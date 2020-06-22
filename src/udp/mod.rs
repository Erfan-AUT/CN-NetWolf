use crate::{dir, tcp, node, BUF_SIZE};
use rand::Rng;
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Mutex, mpsc, mpsc::Receiver};
use std::time::{Duration, Instant};
use std::{thread, time};
pub mod get;

const UDP_SERVER_PORT: u16 = 3222;
const LOCALHOST: &str = "127.0.0.1";
const DISCOVERY_INTERVAL_MS: u64 = 1000;

#[derive(PartialEq)]
enum PacketType {
    Node,
    GETPair,
} 

fn generate_socket() -> UdpSocket {
    let mut current_server_port = UDP_SERVER_PORT;
    loop {
        let udp_server_addr = generate_address(LOCALHOST, current_server_port);
        let _try_socket = match UdpSocket::bind(udp_server_addr) {
            Ok(sckt) => {
                let timeout: Duration = Duration::new(3, 0);
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

fn receive_string_from_socket(socket_mutex: &Mutex<&UdpSocket>) -> Result<(String, SocketAddr), Error> {
    let socket = *socket_mutex.lock().unwrap();
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
    if input_string.starts_with(get::GETPair::header()) {
        return PacketType::GETPair
    }
    PacketType::Node
}

pub async fn discovery_server(receiver: Receiver<String>, socket_mutex: &Mutex<&UdpSocket>, nodes_mutex: &Mutex<&mut HashSet<node::Node>>) {
    let local_address: String;
    let discovery_interval = time::Duration::from_millis(DISCOVERY_INTERVAL_MS);
    {
        let socket = *socket_mutex.lock().unwrap();
        local_address = socket.local_addr().unwrap().to_string();
    }
    loop {
        let mut received_nodes: HashSet<node::Node> = HashSet::new();
        // Read until there are no more incoming disccovery packets.
        loop {
            let data = match receiver.try_recv() {
                Ok(data) => data,
                Err(_) => break
            };
            let mut new_nodes = node::Node::multiple_from_string(data);
            new_nodes.retain(|k| &generate_address(&k.ip.to_string(), k.port) != &local_address);
            received_nodes.extend(new_nodes);
        }
        let mut nodes_ptr = nodes_mutex.lock().unwrap();
        nodes_ptr.extend(received_nodes);
        let nodes = &*nodes_ptr;
        let node_strings = node::Node::nodes_to_string(nodes);
        // Just to make sure the socket's lock get released in the end.
        {
            let socket = *socket_mutex.lock().unwrap();
            for node in &**nodes {
                let _ = match send_bytes_to_socket(node_strings.as_bytes(), node, socket) {
                    Ok(__) => __,
                    Err(_) => continue,
                };
            }
        }
        thread::sleep(discovery_interval);
    }
}

pub async fn get_server(receiver: Receiver<(String, SocketAddr)>, socket_mutex: &Mutex<&UdpSocket>) {


}

pub async fn udp_server(nodes_mutex: Mutex<&mut HashSet<node::Node>>, stdin_rx: Receiver<String>) {
    let socket = generate_socket();
    let socket_mutex = Mutex::new(&socket);
    println!("generated socket successfully!");
    let (discovery_tx, discovery_rx) = mpsc::channel::<String>();
    let (get_tx, get_rx) = mpsc::channel::<(String, SocketAddr)>();
    discovery_server(discovery_rx, &socket_mutex, &nodes_mutex);
    get_server(get_rx, &socket_mutex);
    // Because https://github.com/rust-lang/rfcs/issues/372 is still in the works. :))
    let mut data_addr_pair: (String, SocketAddr);
    loop {
        data_addr_pair = match receive_string_from_socket(&socket_mutex) {
            Ok((string, addr)) => (string, addr),
            Err(_) => continue
        };
        // Because
        if discovery_or_get(&data_addr_pair.0) == PacketType::Node {
            match discovery_tx.send(data_addr_pair.0) {
                Ok(_) => (),
                Err(_) => (),
            };
        }
        else {
            match get_tx.send(data_addr_pair) {
                Ok(_) => (),
                Err(_) => (),
            }
        }
        loop {
            let input = match stdin_rx.try_recv() {
                Ok(data) => data,
                Err(_) => break
            };
            if input.starts_with("list") {
                println!("{:?}", *nodes_mutex.lock().unwrap());
            }
            else if input.starts_with("get") {
                
            }
        }
    }
}
