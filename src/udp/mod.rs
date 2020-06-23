use crate::{dir, node, tcp, BUF_SIZE};
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::sync::{mpsc, mpsc::Receiver, Arc, Mutex};
use std::time::Duration;
use std::{thread, time};

const UDP_SERVER_PORT: u16 = 3222;
const LOCALHOST: &str = "127.0.0.1";
const DISCOVERY_INTERVAL_MS: u64 = 1000;

#[derive(PartialEq)]
enum PacketType {
    DISC,
    GET,
}

fn generate_socket() -> UdpSocket {
    let mut current_server_port = UDP_SERVER_PORT;
    loop {
        let udp_server_addr = generate_address(LOCALHOST, current_server_port);
        let _try_socket = match UdpSocket::bind(udp_server_addr) {
            Ok(sckt) => {
                let timeout: Duration = Duration::new(1, 0);
                sckt.set_write_timeout(Some(timeout)).unwrap();
                sckt.set_read_timeout(Some(timeout)).unwrap();
                return sckt;
            }
            Err(_) => (),
        };
        current_server_port += 1;
    }
}

pub fn generate_address(ip: &str, port: u16) -> String {
    let mut addr = String::from(ip);
    addr.push_str(":");
    addr.push_str(&port.to_string());
    addr
}

fn send_bytes_to_socket(
    data: &[u8],
    node: &node::Node,
    socket: &UdpSocket,
) -> Result<usize, Error> {
    let target_addr = generate_address(&node.ip.to_string(), node.port);
    // Don't really care if it fails.
    socket.send_to(data, target_addr)
}

fn receive_string_from_socket(
    socket_arc: Arc<Mutex<UdpSocket>>,
) -> Result<(String, SocketAddr), Error> {
    let socket_mutex = &*socket_arc;
    let socket = socket_mutex.lock().unwrap();
    let mut buf = [0; BUF_SIZE];
    //This is hyst a ridiculous trick to get over all of rust's size-checking.
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

fn discovery_or_get(input_string: &str) -> PacketType {
    if input_string.starts_with(node::Node::header()) {
        return PacketType::DISC;
    }
    PacketType::GET
}

pub fn discovery_server(
    receiver: Receiver<String>,
    socket_arc: Arc<Mutex<UdpSocket>>,
    nodes_arc: Arc<Mutex<HashSet<node::Node>>>,
) {
    let local_address: String;
    let discovery_interval = time::Duration::from_millis(DISCOVERY_INTERVAL_MS);
    let nodes_mutex = nodes_arc.clone();
    {
        let socket_mutex = socket_arc.lock().unwrap();
        let socket = &*socket_mutex;
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
        let mut nodes_ptr = nodes_mutex.lock().unwrap();
        nodes_ptr.extend(received_nodes);
        println!("{:?}", nodes_ptr);
        let nodes = &*nodes_ptr;
        let node_strings = node::Node::nodes_to_string(nodes);
        // Just to make sure the socket's lock get released in the end.
        {
            let socket_mutex = socket_arc.lock().unwrap();
            let socket = &*socket_mutex;
            for node in nodes {
                let _ = match send_bytes_to_socket(node_strings.as_bytes(), node, socket) {
                    Ok(__) => __,
                    Err(_) => continue,
                };
            }
        }
        thread::sleep(discovery_interval);
    }
}

pub fn GET_ACK_header() -> &'static str {
    "GET/ACK\n"
}

pub fn GET_header() -> &'static str {
    "GET\n"
}

pub fn stdin_GET_header() -> &'static str {
    "get "
}

pub fn get_server(
    receiver: Receiver<(String, SocketAddr)>,
    socket_arc: Arc<Mutex<UdpSocket>>,
    nodes_arc: Arc<Mutex<HashSet<node::Node>>>,
) {
    loop {
        let data_pair: (String, SocketAddr) = match receiver.recv() {
            Ok(data) => data,
            Err(_) => break,
        };
        // If the node is unknown, insert it into our currently known nodes.
        let data = &data_pair.0;
        let addr = &(&data_pair.1).to_string();
        // Wanted to put this entire sneaky node shenanigan in an inline function,
        // But apparently rust's inline functions are just not really good:
        // https://github.com/rust-lang/rust/issues/14527
        let mut current_node = &node::Node::short_single_from_string(addr);
        let nodes_mutex = nodes_arc.clone();
        let nodes_ptr = nodes_mutex.lock().unwrap();
        let nodes = &*nodes_ptr;
        for node in nodes {
            if node.is_sneaky_node(addr) {
                current_node = node;
                break;
            }
        }

        let mut data_lines = data.lines();
        // Send ACK to GET request
        if data_lines.next().unwrap().starts_with(GET_header()) {
            // Becomes useless, so why should it keep the mutex?
            drop(nodes);
            let file_name = &data_lines.next().unwrap();
            // Don't respond if you don't have the file!
            // For the reason why "contains" is not used, please refer to:
            // https://github.com/rust-lang/rust/issues/42671
            if dir::file_list().iter().any(|x| x == file_name) {
                let mut response = String::from(GET_ACK_header());
                response.push_str(&crate::TCP_PORT.to_string());
                response.push('\n');
                response.push_str(&file_name);
                let socket_mutex = socket_arc.lock().unwrap();
                let socket = &*socket_mutex;
                let _ = match send_bytes_to_socket(
                    response.to_string().as_bytes(),
                    &current_node,
                    socket,
                ) {
                    Ok(__) => __,
                    Err(_) => return,
                };
                // This one's too functional programming-y. :)))
                let mut current_clone = current_node.clone();
                current_clone.prior_communications += 1;
                // This is to signify that our previous immutable borrow is invalid from now on.
                let mut nodes_ptr = nodes_mutex.lock().unwrap();
                nodes_ptr.remove(current_node);
                nodes_ptr.insert(current_clone);
                let prior_node_comms = current_node.prior_communications;
                // Unlock the mutex because it's not needed anymore, but it'll linger on for too long.
                drop(nodes_ptr);
                println!("Starting TCP Send server");
                let addr_string = addr.to_string();
                let file_name_string = file_name.to_string();
                std::thread::spawn(move || {
                    tcp::tcp_get_sender(addr_string, file_name_string, prior_node_comms)
                });
                // tcp::tcp_get_sender(file_name, prior_node_comms).unwrap();
            }
        }
        // Connect to a node that has ACK'd one of your previous requests.
        else {
            let mut tcp_socket_addr = data_pair.1.clone();
            let port_str = data_lines.next().unwrap();
            tcp_socket_addr.set_port(port_str.parse::<u16>().unwrap());
            let file_name = data_lines.next().unwrap().to_string();
            std::thread::spawn(move || tcp::tcp_get_receiver(tcp_socket_addr.clone(), file_name));
        }
    }
}

pub fn get_client(
    receiver: Receiver<String>,
    socket_arc: Arc<Mutex<UdpSocket>>,
    nodes_arc: Arc<Mutex<HashSet<node::Node>>>,
) {
    loop {
        let input = match receiver.recv() {
            Ok(data) => data,
            Err(_) => continue,
        };
        let mut commands = input.split(" ");
        let arg = commands.next().unwrap();
        if arg.starts_with("list") {
            let value = nodes_arc.lock().unwrap();
            println!("{:?}", value);
        } else if arg.starts_with(stdin_GET_header()) {
            // Make sure there is a file name!
            let file_name = match commands.next() {
                Some(cmd) => cmd,
                None => continue,
            };
            let socket_mutex = socket_arc.lock().unwrap();
            let socket = &*socket_mutex;
            let nodes_ptr = nodes_arc.lock().unwrap();
            let nodes = &*nodes_ptr;
            for node in nodes {
                let mut request = String::from(GET_header());
                request.push('\n');
                request.push_str(file_name);
                send_bytes_to_socket(request.as_bytes(), node, socket).unwrap_or(0);
            }
        }
    }
}

pub fn udp_server(init_nodes_dir: String, stdin_rx: Receiver<String>) {
    // The fact whether or not this actually gets updated is still a question. :)))
    let nodes = node::read_starting_nodes(&init_nodes_dir);
    let nodes_mutex = Mutex::new(nodes);
    let socket = generate_socket();
    let socket_mutex = Mutex::new(socket);
    let socket_arc = Arc::new(socket_mutex);
    let nodes_arc = Arc::new(nodes_mutex);
    println!("generated socket successfully!");
    let (discovery_tx, discovery_rx) = mpsc::channel::<String>();
    let (get_tx, get_rx) = mpsc::channel::<(String, SocketAddr)>();
    //Spawn the clones first kids! Don't do it while calling the function. :)))))))
    let socket_arc_disc_clone = socket_arc.clone();
    let node_arc_disc_clone = nodes_arc.clone();
    thread::spawn(|| discovery_server(discovery_rx, socket_arc_disc_clone, node_arc_disc_clone));
    let socket_arc_get_clone = socket_arc.clone();
    let node_arc_get_clone = nodes_arc.clone();
    thread::spawn(|| get_server(get_rx, socket_arc_get_clone, node_arc_get_clone));
    let socket_arc_get_client_clone = socket_arc.clone();
    let node_arc_get_client_clone = nodes_arc.clone();
    std::thread::spawn(|| {
        get_client(
            stdin_rx,
            socket_arc_get_client_clone,
            node_arc_get_client_clone,
        )
    });
    // Because https://github.com/rust-lang/rfcs/issues/372 is still in the works. :))
    let mut data_addr_pair: (String, SocketAddr);
    loop {
        data_addr_pair = match receive_string_from_socket(socket_arc.clone()) {
            Ok((string, addr)) => (string, addr),
            Err(_) => continue,
        };
        if discovery_or_get(&data_addr_pair.0) == PacketType::DISC {
            match discovery_tx.send(data_addr_pair.0) {
                Ok(_) => (),
                Err(_) => (),
            };
        } else {
            match get_tx.send(data_addr_pair) {
                Ok(_) => (),
                Err(_) => (),
            }
        }
    }
}
