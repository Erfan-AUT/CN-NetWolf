use crate::node;
use rand::Rng;
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::sync::Mutex;
use std::time::{Duration, Instant};
mod get;

const UDP_SERVER_PORT: i32 = 3222;
const LOCALHOST: &str = "127.0.0.1";
const BUF_SIZE: usize = 8192;
const REFRESH_INTERVAL_MS: u128 = 1000;

fn generate_socket() -> UdpSocket {
    let mut current_server_port = UDP_SERVER_PORT;
    loop {
        let udp_server_addr = generate_address(LOCALHOST, current_server_port);
        let _try_socket = match UdpSocket::bind(udp_server_addr) {
            Ok(sckt) => return sckt,
            Err(_) => (),
        };
        current_server_port += 1;
    }
}

pub fn generate_address(ip: &str, port: i32) -> String {
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

fn receive_string_from_socket(socket: &UdpSocket) -> Result<(String, SocketAddr), Error> {
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

fn udp_discovery_server(socket: &UdpSocket, mutex: &Mutex<&mut HashSet<node::Node>>) {
    let local_address = socket.local_addr().unwrap().to_string();
    let mut nodes_ptr = mutex.lock().unwrap();
    let nodes = &*nodes_ptr;
    let node_strings = node::Node::nodes_to_string(nodes);
    let mut received_nodes: HashSet<node::Node> = HashSet::new();
    // De-reference and referencing so that it could be iterated over.
    // I swear I'm just playing around with pointers until it gives up. :)))))
    for node in &**nodes {
        let _ = match send_bytes_to_socket(node_strings.as_bytes(), node, socket) {
            Ok(__) => __,
            Err(_) => continue,
        };
        let (received_nodes_str, _) = match receive_string_from_socket(socket) {
            Ok((string, __)) => (string, __),
            Err(_) => continue,
        };
        println!("{}", received_nodes_str);
        let mut new_nodes = node::Node::multiple_from_string(received_nodes_str);
        // Removes it if the receiving node itself is encountered
        new_nodes.retain(|k| &generate_address(&k.ip.to_string(), k.port) != &local_address);
        received_nodes.extend(new_nodes);
    }
    nodes_ptr.extend(received_nodes);
}

async fn udp_get_server(socket: &UdpSocket, mutex: &Mutex<&mut HashSet<node::Node>>) {
    let mut rng = rand::thread_rng();
    let is_request = rng.gen::<bool>();
    if is_request {
        udp_get_requester(socket, mutex).await;
    } else {
        // Don't respond if you don't have the file!
        udp_get_responder(socket).await;
    }
}

async fn udp_get_responder(socket: &UdpSocket) {
    let (result_string, src) = match receive_string_from_socket(socket) {
        Ok((string, src)) => (string, src),
        Err(_) => return,
    };
    let request = match get::GETPair::from_str(&result_string) {
        Ok(req) => req,
        Err(_) => return,
    };
    let target = node::Node::new("", &src.to_string(), request.tcp_port);
    let response = get::GETPair::with_random_port(&request.file_name);
    // Sending ACK
    let _ = match send_bytes_to_socket(response.to_string().as_bytes(), &target, socket) {
        Ok(__) => __,
        Err(_) => return,
    };
}

async fn udp_get_requester(socket: &UdpSocket, mutex: &Mutex<&mut HashSet<node::Node>>) {
    let nodes_ptr = mutex.lock().unwrap();
    let nodes = &*nodes_ptr;
    let mut min_duration = Duration::new(3, 0);
    let mut tcp_pair: (String, get::GETPair);
    let request = get::GETPair::random_get();
    for node in &**nodes {
        let start_time = Instant::now();
        // Requesting a file from another node
        if request.file_name != "null" {
            let _ = match send_bytes_to_socket(request.to_string().as_bytes(), node, socket) {
                Ok(__) => __,
                Err(_) => continue,
            };
            let (get_pair_string, src) = match receive_string_from_socket(socket) {
                Ok((string, src)) => (string, src),
                Err(_) => continue,
            };
            let duration = start_time.elapsed();
            let response = match get::GETPair::from_str(&get_pair_string) {
                Ok(res) => res,
                Err(_) => continue,
            };
            // Extra checking! :))
            if min_duration > duration && request.file_name == response.file_name {
                tcp_pair = (src.ip().to_string(), response);
                min_duration = duration;
            }
        }
    }
    // Checking that there was an ACK.
    if Duration::new(3, 0) > min_duration {
        // Spawn TCP
        println!("ooh yeaah");
    }
}

pub async fn udp_server(mutex: Mutex<&mut HashSet<node::Node>>) {
    let socket = generate_socket();
    println!("generated socket successfully!");
    let mut start_time = Instant::now();
    loop {
        let duration = start_time.elapsed();
        if duration.as_millis() > REFRESH_INTERVAL_MS {
            udp_discovery_server(&socket, &mutex);
            start_time = Instant::now();
            continue;
        }
        udp_get_server(&socket, &mutex).await;
    }
}
