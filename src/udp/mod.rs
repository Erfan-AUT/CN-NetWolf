use crate::node;
use std::collections::HashSet;
use std::net::{UdpSocket, SocketAddr};
use std::sync::Mutex;
// use std::thread;
use rand::Rng;
use std::time::{Instant, Duration};
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

fn trim_buffer(buf: &[u8]) -> &[u8] {
    let mut first_index: usize = 0;
    for i in 0..buf.len() {
        if buf[i] as char == '\0' {
            first_index = i;
            break;
        }
    }
    &buf[0..first_index]
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
            Ok(_) => match std::str::from_utf8(&buf) {
                Ok(node_strs) => node_strs.to_string(),
                Err(_) => continue,
            },
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

fn generate_get() -> get::GETRequest {
    let request_options = ["mamad.txt", "reza.mp4", "ahmad.png"];
    let mut rng = rand::thread_rng();
    let option_index = rng.gen_range(0, request_options.len());
    let random_option = request_options[option_index];
    let request = get::GETRequest::new(random_option);
    request
}

fn udp_get_server(socket: &UdpSocket, mutex: &Mutex<&mut HashSet<node::Node>>) {
    let nodes_ptr = mutex.lock().unwrap();
    let nodes = &*nodes_ptr;
    let request = generate_get();
    let mut min_duration = Duration::new(3, 0);
    let mut udp_pair: (usize, SocketAddr, get::GETResponse);
    for node in &**nodes {
        let target_addr = generate_address(&node.ip.to_string(), node.port);
        let start_time = Instant::now();
        socket
            .send_to(request.to_string().as_bytes(), target_addr)
            .ok();
        let mut buf = [0; BUF_SIZE];
        let (amt, src) = match socket.recv_from(&mut buf) {
            Ok((amt, src)) => (amt, src),
            Err(_) => continue
        };
        let duration = start_time.elapsed();
        if min_duration > duration {
            udp_pair = (amt, src, get::GETResponse::new(1));
            min_duration = duration;
        }
        let buf = trim_buffer(&buf);
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
        }
    }
}
