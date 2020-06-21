use crate::node;
use std::collections::HashSet;
use std::net::UdpSocket;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

const UDP_SERVER_PORT: i32 = 3321;
const LOCALHOST: &str = "127.0.0.1:";
const BUF_SIZE: usize = 8192;
const REFRESH_INTERVAL_S: u64 = 1;

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

pub async fn udp_discovery_server(mutex: Mutex<&mut HashSet<node::Node>>) {
    let socket = generate_socket();
    let local_address = socket.local_addr().unwrap().to_string();
    // I swear I'm just playing around with pointers until it gives up. :)))))
    loop {
        {
            let nodes = &*mutex.lock().unwrap();
            let node_strings = node::Node::nodes_to_string(nodes);
            let mut received_nodes: HashSet<node::Node> = HashSet::new();
            // De-reference and referencing so that it could be iterated over.
            for node in &**nodes {
                let node_bytes = node_strings.as_bytes();
                let target_addr = generate_address(&node.ip.to_string(), node.port);
                // Don't really care if it fails.
                socket.send_to(node_bytes, target_addr).ok();
                let mut buf = [0; BUF_SIZE];
                let rcv_result = socket.recv_from(&mut buf);
                let received_nodes_str = match rcv_result {
                    Ok(_) => match std::str::from_utf8(&buf) {
                        Ok(node_strs) => node_strs.to_string(),
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                };
                let mut new_nodes = node::Node::multiple_from_string(received_nodes_str);
                // Removes it if the receiving node itself is encountered
                new_nodes.retain(|k| &generate_address(&k.ip.to_string(), k.port) != &local_address);
                received_nodes.extend(new_nodes);
            }
            thread::sleep(Duration::from_secs(REFRESH_INTERVAL_S));
        }
    }
}
