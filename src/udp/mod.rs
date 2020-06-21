use crate::node;
use std::collections::HashSet;
use std::net::UdpSocket;
use std::sync::Mutex;
// use std::thread;
use std::convert::TryInto;
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

fn udp_discovery_server(socket: &UdpSocket, mutex: &Mutex<&mut HashSet<node::Node>>) {
    let local_address = socket.local_addr().unwrap().to_string();
    let mut nodes_ptr = mutex.lock().unwrap();
    let nodes = &*nodes_ptr;
    let node_strings = node::Node::nodes_to_string(nodes);
    let mut received_nodes: HashSet<node::Node> = HashSet::new();
    // De-reference and referencing so that it could be iterated over.
    // I swear I'm just playing around with pointers until it gives up. :)))))
    for node in &**nodes {
        let node_bytes = node_strings.as_bytes();
        let target_addr = generate_address(&node.ip.to_string(), node.port);
        // Don't really care if it fails.
        socket.send_to(node_bytes, target_addr).ok();
        let mut buf = [0; BUF_SIZE];
        let rcv_result = socket.recv_from(&mut buf);
        let buf = trim_buffer(&buf);
        let received_nodes_str = match rcv_result {
            Ok((amt, _)) => std::str::from_utf8(&buf[..amt]).unwrap().to_string(),
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
    let nodes_ptr = mutex.lock().unwrap();
    let nodes = &*nodes_ptr;
    let request = get::GETRequest::random_get();
    let mut min_duration = Duration::new(3, 0);
    let mut udp_triplet: (String, get::GETResponse);
    for node in &**nodes {
        let target_addr = generate_address(&node.ip.to_string(), node.port);
        let start_time = Instant::now();
        socket
            .send_to(request.to_string().as_bytes(), target_addr)
            .ok();
        let mut buf = [0; BUF_SIZE];
        let (amt, src) = match socket.recv_from(&mut buf) {
            Ok((amt, src)) => (amt, src),
            Err(_) => continue,
        };
        let tcp_port = i32::from_be_bytes(match buf[..amt].try_into() {
            Ok(arr) => arr,
            Err(_) => continue,
        });
        let res = get::GETResponse::new(tcp_port);
        let duration = start_time.elapsed();
        if min_duration > duration {
            udp_triplet = (src.ip().to_string(), res);
            min_duration = duration;
        }
    }
}

pub async fn udp_server(mutex: Mutex<&mut HashSet<node::Node>>) {
    let socket = generate_socket();
    println!("generated socket successfully!");
    let mut start_time = Instant::now();
    loop {
        {
            let duration = start_time.elapsed();
            if duration.as_millis() > REFRESH_INTERVAL_MS {
                udp_discovery_server(&socket, &mutex);
                start_time = Instant::now();
                continue;
            }
            udp_get_server(&socket, &mutex).await;
        }
    }
}
