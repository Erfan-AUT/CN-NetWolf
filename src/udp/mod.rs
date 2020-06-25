use crate::tcp::tcp_server;
use crate::{dir, node, tcp, BUF_SIZE, CURRENT_DATA_CLIENTS, MAX_DATA_CLIENTS};
use log::info;
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::sync::{mpsc, mpsc::Receiver, Arc, RwLock};
use std::time::Duration;
use std::{thread, time};
pub mod headers;
mod reliable;

const UDP_SERVER_PORT: u16 = 3222;
const LOCALHOST: &str = "127.0.0.1";
const DISCOVERY_INTERVAL_MS: u64 = 1000;

pub fn bind_udp_socket(mut port: u16) -> UdpSocket {
    loop {
        let udp_server_addr = generate_address(LOCALHOST, port);
        match UdpSocket::bind(udp_server_addr) {
            Ok(sckt) => {
                let timeout: Duration = Duration::new(1, 0);
                sckt.set_write_timeout(Some(timeout)).unwrap();
                sckt.set_read_timeout(Some(timeout)).unwrap();
                return sckt;
            }
            Err(_) => (),
        };
        // Try another port if the previous port failed
        port += 1;
    }
}

pub fn generate_address(ip: &str, port: u16) -> String {
    let mut addr = String::from(ip);
    addr.push_str(":");
    addr.push_str(&port.to_string());
    addr
}

fn send_bytes_to_udp_socket(
    data: &[u8],
    node: &node::Node,
    socket: &UdpSocket,
) -> Result<usize, Error> {
    let target_addr = generate_address(&node.ip.to_string(), node.port);
    // Don't really care if it fails.
    socket.send_to(data, target_addr)
}

fn receive_string_from_udp_socket(
    socket_arc: Arc<RwLock<UdpSocket>>,
) -> Result<(String, SocketAddr), Error> {
    let socket_arc = socket_arc.clone();
    let socket = &*socket_arc.read().unwrap();
    let mut buf = [0; BUF_SIZE];
    //This is just a ridiculous trick to get over all of rust's size-checking.
    let err = Error::new(ErrorKind::Other, "OH NONONO");
    let (amt, src) = match socket.recv_from(&mut buf) {
        Ok((amt, src)) => (amt, src),
        Err(e) => return Err(e),
    };
    //This is where the data is fully received
    match std::str::from_utf8(&buf[..amt]) {
        Ok(string) => Ok((string.to_string(), src)),
        Err(_) => Err(err),
    }
}

pub fn discovery_server(
    receiver: Receiver<String>,
    socket_arc: Arc<RwLock<UdpSocket>>,
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
) {
    let local_address: String;
    let discovery_interval = time::Duration::from_millis(DISCOVERY_INTERVAL_MS);
    let nodes_rwlock = nodes_arc.clone();
    {
        let socket_rwlock = socket_arc.read().unwrap();
        let socket = &*socket_rwlock;
        local_address = socket.local_addr().unwrap().to_string();
    }
    loop {
        let mut received_nodes: HashSet<node::Node> = HashSet::new();
        // Read until there are no more incoming disccovery packets.
        // This should not wait for data and do its job indefinitely.
        loop {
            let data = match receiver.try_recv() {
                Ok(data) => data,
                Err(_) => break,
            };
            let mut new_nodes = node::Node::multiple_from_string(data, true);
            new_nodes.retain(|k| &generate_address(&k.ip.to_string(), k.port) != &local_address);
            received_nodes.extend(new_nodes);
        }
        let mut nodes_ptr = nodes_rwlock.write().unwrap();
        nodes_ptr.extend(received_nodes);
        drop(nodes_ptr);
        let nodes_ptr = nodes_rwlock.read().unwrap();
        let nodes = &*nodes_ptr;
        let node_strings = node::Node::nodes_to_string(nodes);
        // Just to make sure the socket's lock get released in the end.
        {
            let socket_rwlock = socket_arc.write().unwrap();
            let socket = &*socket_rwlock;
            for node in nodes {
                let _ = match send_bytes_to_udp_socket(node_strings.as_bytes(), node, socket) {
                    Ok(__) => __,
                    Err(_) => continue,
                };
            }
        }
        drop(nodes_ptr);
        thread::sleep(discovery_interval);
    }
}

// Wanted to put this entire sneaky node shenanigan in an inline function,
// But apparently rust's inline functions are just not really good:
// https://github.com/rust-lang/rust/issues/14527
pub fn node_of_packet(
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
    sneaky_arc: Arc<RwLock<u16>>,
    addr: &str,
) -> (node::Node, bool) {
    let sneaky_rw = sneaky_arc.clone();
    let mut sneaky_count = sneaky_rw.write().unwrap();
    *sneaky_count += 1;
    let mut was_sneaky = true;
    let mut current_node = node::Node::new_sneaky(addr, *sneaky_rw.read().unwrap());
    let nodes_rwlock = nodes_arc.clone();
    let nodes_ptr = nodes_rwlock.read().unwrap();
    for node in &*nodes_ptr {
        if node.has_same_address(addr) {
            *sneaky_count -= 1;
            was_sneaky = false;
            current_node = node.clone();
            break;
        }
    }
    (current_node, was_sneaky)
}

pub fn get_server(
    receiver: Receiver<(String, SocketAddr)>,
    socket_arc: Arc<RwLock<UdpSocket>>,
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
    sneaky_arc: Arc<RwLock<u16>>,
) {
    loop {
        let data_pair: (String, SocketAddr) = match receiver.recv() {
            Ok(data) => data,
            Err(_) => break,
        };
        // If the node is unknown, insert it into our currently known nodes.
        let data = &data_pair.0;
        let addr = &(&data_pair.1).to_string();
        let (current_node, _) = node_of_packet(nodes_arc.clone(), sneaky_arc.clone(), addr);

        let mut data_lines = data.lines();
        // Send ACK to GET request
        if data_lines
            .next()
            .unwrap()
            .starts_with(headers::PacketHeader::get().trim())
        {
            // Becomes useless, so why should it keep the rwlock?
            let file_name = &data_lines.next().unwrap();
            let client_count = *CURRENT_DATA_CLIENTS.read().unwrap();
            // Don't respond if you don't have the file ot the TCP Server is swamped with too many clients.
            // For the reason why "contains" is not used, please refer to:
            // https://github.com/rust-lang/rust/issues/42671
            if dir::file_list().iter().any(|x| x == file_name) && MAX_DATA_CLIENTS > client_count {
                info!("Recognizing the existence of the requested file.");
                let mut response = String::from(headers::PacketHeader::ack());
                response.push_str(&crate::DATA_PORT.to_string());
                response.push('\n');
                // Because the node might not remember what it requested! :))
                response.push_str(&file_name);
                info!("The proper response is: {}", response);
                let socket_rwlock = socket_arc.write().unwrap();
                let socket = &*socket_rwlock;
                let _ = match send_bytes_to_udp_socket(
                    response.to_string().as_bytes(),
                    &current_node,
                    socket,
                ) {
                    Ok(__) => (),
                    Err(_) => continue,
                };
                drop(socket_rwlock);
                info!("No problem sending GET/ACK over UDP Socket.");
            } else {
                info!("File not found, denying the GET request");
            }
        }
        // Connect to a node that has ACK'd one of your previous requests.
        else {
            let mut tcp_socket_addr = data_pair.1.clone();
            let port_str = data_lines.next().unwrap();
            info!("Port string is: {}", port_str);
            tcp_socket_addr.set_port(port_str.parse::<u16>().unwrap());
            let file_name = data_lines.next().unwrap().to_string();
            info!("Starting TCP Receive Client");
            std::thread::spawn(move || tcp::tcp_client(tcp_socket_addr.clone(), file_name));
        }
    }
}

pub fn get_client(
    receiver: Receiver<String>,
    socket_arc: Arc<RwLock<UdpSocket>>,
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
) {
    loop {
        let input = match receiver.recv() {
            Ok(data) => data,
            Err(_) => continue,
        };
        info!("Received data");
        let mut commands = input.split(" ");
        let arg = commands.next().unwrap();

        if arg.starts_with(headers::StdinHeader::list()) {
            let value = &*nodes_arc.read().unwrap();
            println!("{:?}", value);
        } else if arg.starts_with(headers::StdinHeader::get()) {
            info!("Understand GET");
            // Make sure there is a file name!
            let file_name = match commands.next() {
                Some(cmd) => cmd.trim(),
                None => continue,
            };
            info!("Waiting for socket acq.");
            let socket_rwlock = socket_arc.write().unwrap();
            let socket = &*socket_rwlock;
            info!("Waiting for nodes acq.");
            let nodes_ptr = nodes_arc.read().unwrap();
            let nodes = &*nodes_ptr;
            info!("Preparing to broadcast GET");
            for node in nodes {
                info!("GET sent to {}", node);
                let mut request = String::from(headers::PacketHeader::get());
                request.push_str(file_name);
                info!("The request is: {}", request);
                send_bytes_to_udp_socket(request.as_bytes(), node, socket).unwrap_or(0);
            }
        }
    }
}

pub fn main_server(init_nodes_dir: String, stdin_rx: Receiver<String>) {
    // The fact whether or not this actually gets updated is still a question. :)))
    let nodes = node::read_starting_nodes(&init_nodes_dir);
    let nodes_rwlock = RwLock::new(nodes);
    let socket = bind_udp_socket(UDP_SERVER_PORT);
    let socket_rwlock = RwLock::new(socket);
    let socket_arc = Arc::new(socket_rwlock);
    let nodes_arc = Arc::new(nodes_rwlock);
    let sneaky_count: u16 = 0;
    let sneaky_rwlock = RwLock::new(sneaky_count);
    let sneaky_arc = Arc::new(sneaky_rwlock);
    info!("Generated UDP socket successfully!");
    let (discovery_tx, discovery_rx) = mpsc::channel::<String>();
    let (get_server_tx, get_server_rx) = mpsc::channel::<(String, SocketAddr)>();
    //Spawn the clones first kids! Don't do it while calling the function. :)))))))
    let socket_arc_disc_clone = socket_arc.clone();
    let node_arc_disc_clone = nodes_arc.clone();
    thread::spawn(|| discovery_server(discovery_rx, socket_arc_disc_clone, node_arc_disc_clone));
    let socket_arc_get_server = socket_arc.clone();
    let nodes_arc_get_server = nodes_arc.clone();
    let sneaky_arc_get_server = sneaky_arc.clone();
    thread::spawn(|| {
        get_server(
            get_server_rx,
            socket_arc_get_server,
            nodes_arc_get_server,
            sneaky_arc_get_server,
        )
    });
    let socket_arc_get_client = socket_arc.clone();
    let nodes_arc_get_client = nodes_arc.clone();
    std::thread::spawn(|| get_client(stdin_rx, socket_arc_get_client, nodes_arc_get_client));
    let nodes_arc_data_server = nodes_arc.clone();
    let sneaky_arc_data_server = sneaky_arc.clone();
    thread::spawn(|| tcp_server(nodes_arc_data_server, sneaky_arc_data_server));
    // Because https://github.com/rust-lang/rfcs/issues/372 is still in the works. :))
    let mut data_addr_pair: (String, SocketAddr);
    loop {
        // This function is the only one reading from the socket!
        data_addr_pair = match receive_string_from_udp_socket(socket_arc.clone()) {
            Ok((string, addr)) => (string, addr),
            Err(_) => continue,
        };
        let header = headers::PacketHeader::packet_type(&data_addr_pair.0);
        if header == headers::PacketHeader::Disc {
            match discovery_tx.send(data_addr_pair.0) {
                Ok(_) => (),
                Err(_) => (),
            };
        } else if header == headers::PacketHeader::GETACK {
            match get_server_tx.send(data_addr_pair) {
                Ok(_) => (),
                Err(_) => (),
            }
        } else {
            info!("Packet was not recognized!");
        }
    }
}
