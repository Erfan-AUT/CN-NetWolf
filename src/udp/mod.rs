use crate::networking::{
    self, bind_udp_socket, ip_port_string, node_of_packet, BUF_SIZE, CURRENT_DATA_CLIENTS,
    DISCOVERY_INTERVAL_MS, MAX_DATA_CLIENTS, UDP_GET_PORT,
};
use crate::tcp::tcp_server;
use crate::DATA_CONN_TYPE;
use crate::{dir, node, tcp};
use log::info;
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::sync::{mpsc, mpsc::Receiver, Arc, RwLock};
use std::{thread, time};
pub mod headers;
mod reliable;

fn send_bytes_to_udp_socket(
    data: &[u8],
    node: &node::Node,
    socket: &UdpSocket,
) -> Result<usize, Error> {
    let target_addr = ip_port_string(node.ip, node.port);
    // Don't really care if it fails.
    socket.send_to(data, target_addr)
}

fn receive_string_from_udp_socket(socket: &UdpSocket) -> Result<(String, SocketAddr), Error> {
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
    socket: UdpSocket,
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
) {
    let discovery_interval = time::Duration::from_millis(DISCOVERY_INTERVAL_MS);
    let nodes_rwlock = nodes_arc.clone();
    let local_address = socket.local_addr().unwrap().to_string();
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
            new_nodes.retain(|k| ip_port_string(k.ip, k.port) != local_address);
            received_nodes.extend(new_nodes);
        }
        let mut nodes_ptr = nodes_rwlock.write().unwrap();
        nodes_ptr.extend(received_nodes);
        drop(nodes_ptr);
        let nodes_ptr = nodes_rwlock.read().unwrap();
        let nodes = &*nodes_ptr;
        let node_strings = node::Node::nodes_to_string(nodes);
        for node in nodes {
            let _ = match send_bytes_to_udp_socket(node_strings.as_bytes(), node, &socket) {
                Ok(__) => __,
                Err(_) => continue,
            };
        }
        drop(nodes_ptr);
        thread::sleep(discovery_interval);
    }
}

pub fn get_server(
    receiver: Receiver<(String, SocketAddr)>,
    socket: UdpSocket,
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
) {
    loop {
        let data_pair: (String, SocketAddr) = match receiver.recv() {
            Ok(data) => data,
            Err(_) => break,
        };
        // If the node is unknown, insert it into our currently known nodes.
        let data = &data_pair.0;
        let addr = &(&data_pair.1).to_string();
        info!("Received {} from {}", data, addr);
        let (current_node, _) = node_of_packet(nodes_arc.clone(), addr);
        info!("Recognized node's packet.");
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
            info!("All is fine this far.");
            // Don't respond if you don't have the file ot the TCP Server is swamped with too many clients.
            // For the reason why "contains" is not used, please refer to:
            // https://github.com/rust-lang/rust/issues/42671
            if dir::file_list().iter().any(|x| x == file_name) && MAX_DATA_CLIENTS > client_count {
                info!("Recognizing the existence of the requested file.");
                let mut response = String::from(headers::PacketHeader::ack());
                response.push_str(&networking::DATA_SENDER_PORT.to_string());
                response.push('\n');
                // Because the node might not remember what it requested! :))
                response.push_str(&file_name);
                info!("The proper response is: {}", response);
                let _ = match send_bytes_to_udp_socket(
                    response.to_string().as_bytes(),
                    &current_node,
                    &socket,
                ) {
                    Ok(__) => (),
                    Err(_) => continue,
                };
                info!("No problem sending GET/ACK over UDP Socket.");
            } else {
                info!("File not found, denying the GET request");
            }
        }
        // Connect to a node that has ACK'd one of your previous requests.
        else {
            let mut data_socket_addr = data_pair.1.clone();
            let port_str = data_lines.next().unwrap();
            data_socket_addr.set_port(port_str.parse::<u16>().unwrap());
            let file_name = data_lines.next().unwrap().to_string();
            match *DATA_CONN_TYPE.read().unwrap() {
                headers::ConnectionType::TCP => {
                    thread::spawn(move || tcp::tcp_client(data_socket_addr.clone(), file_name));
                }
                headers::ConnectionType::SAndW => {
                    thread::spawn(move || reliable::sw_client(data_socket_addr.clone(), file_name));
                }
                headers::ConnectionType::GoBackN => {
                    thread::spawn(move || tcp::tcp_client(data_socket_addr.clone(), file_name));
                }
                headers::ConnectionType::SRepeat => {
                    thread::spawn(move || tcp::tcp_client(data_socket_addr.clone(), file_name));
                }
            };
        }
    }
}

pub fn get_client(
    receiver: Receiver<String>,
    socket: UdpSocket,
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
            info!("Waiting for nodes acq.");
            let nodes_ptr = nodes_arc.read().unwrap();
            let nodes = &*nodes_ptr;
            info!("Preparing to broadcast GET");
            for node in nodes {
                info!("GET sent to {}", node);
                let mut request = String::from(headers::PacketHeader::get());
                request.push_str(file_name);
                info!("The request is: {}", request);
                send_bytes_to_udp_socket(request.as_bytes(), node, &socket).unwrap_or(0);
            }
        }
    }
}

pub fn main_server(init_nodes_dir: String, stdin_rx: Receiver<String>) {
    // The fact whether or not this actually gets updated is still a question. :)))
    let nodes = node::read_starting_nodes(&init_nodes_dir);
    let nodes_rwlock = RwLock::new(nodes);
    let socket = bind_udp_socket(UDP_GET_PORT);
    let nodes_arc = Arc::new(nodes_rwlock);
    info!("Generated UDP socket successfully!");
    let (discovery_tx, discovery_rx) = mpsc::channel::<String>();
    let (get_server_tx, get_server_rx) = mpsc::channel::<(String, SocketAddr)>();
    //Spawn the clones first kids! Don't do it while calling the function. :)))))))
    let socket_disc = socket.try_clone().unwrap();
    let node_arc_disc_clone = nodes_arc.clone();
    thread::spawn(|| discovery_server(discovery_rx, socket_disc, node_arc_disc_clone));
    let socket_get_server = socket.try_clone().unwrap();
    let nodes_arc_get_server = nodes_arc.clone();
    thread::spawn(|| get_server(get_server_rx, socket_get_server, nodes_arc_get_server));
    let socket_get_client = socket.try_clone().unwrap();
    let nodes_arc_get_client = nodes_arc.clone();
    std::thread::spawn(|| get_client(stdin_rx, socket_get_client, nodes_arc_get_client));
    let nodes_arc_data_server = nodes_arc.clone();
    match *DATA_CONN_TYPE.read().unwrap() {
        headers::ConnectionType::TCP => {
            thread::spawn(|| tcp_server(nodes_arc_data_server));
        }
        headers::ConnectionType::SAndW => {
            thread::spawn(|| reliable::sw_server(nodes_arc_data_server));
        }
        headers::ConnectionType::GoBackN => {
            thread::spawn(|| tcp_server(nodes_arc_data_server));
        }
        headers::ConnectionType::SRepeat => {
            thread::spawn(|| tcp_server(nodes_arc_data_server));
        }
    };
    // Because https://github.com/rust-lang/rfcs/issues/372 is still in the works. :))
    let mut data_addr_pair: (String, SocketAddr);
    loop {
        // This function is the only one reading from the socket!
        data_addr_pair = match receive_string_from_udp_socket(&socket) {
            Ok((string, addr)) => (string, addr),
            Err(_) => continue,
        };
        let header = headers::PacketHeader::packet_type(&data_addr_pair.0);
        if header == headers::PacketHeader::Disc {
            match discovery_tx.send(data_addr_pair.0) {
                Ok(_) => (),
                Err(_) => (),
            };
        } else if header == headers::PacketHeader::GETACK || header == headers::PacketHeader::GET {
            match get_server_tx.send(data_addr_pair) {
                Ok(_) => (),
                Err(_) => (),
            }
        } else {
            info!("Packet was not recognized!");
        }
    }
}
