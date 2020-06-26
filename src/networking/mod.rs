use crate::node;
use rand::Rng;
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::net::{Ipv4Addr, UdpSocket};
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub const CONGESTION_DELAY_MS: u64 = 500;
pub const LOCALHOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
pub const UDP_GET_PORT: u16 = 3222;
pub const DISCOVERY_INTERVAL_MS: u64 = 1000;
pub const BUF_SIZE: usize = 8192;
pub const MAX_DATA_CLIENTS: u16 = 3;
pub const PORT_MIN: u16 = 2000;
pub const PORT_MAX: u16 = 5000;

lazy_static! {
    pub static ref CURRENT_DATA_CLIENTS: RwLock<u16> = RwLock::new(0);
    pub static ref DATA_SENDER_PORT: u16 = random_data_port();
    pub static ref DATA_RECEIVER_PORT: u16 = random_data_port();
}

fn random_data_port() -> u16 {
    let mut r = rand::thread_rng();
    r.gen_range(PORT_MIN, PORT_MAX)
}
// Wanted to put this entire sneaky node shenanigan in an inline function,
// But apparently rust's inline functions are just not really good:
// https://github.com/rust-lang/rust/issues/14527
pub fn node_of_packet(
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
    addr: &str,
) -> (node::Node, bool) {
    let mut was_sneaky = true;
    let mut current_node = node::Node::new_sneaky(addr);
    let nodes_rwlock = nodes_arc.clone();
    let nodes_ptr = nodes_rwlock.read().unwrap();
    info!("Trying to unlock nodes rw");
    for node in &*nodes_ptr {
        if node.has_same_address(addr) {
            was_sneaky = false;
            current_node = node.clone();
            break;
        }
    }
    (current_node, was_sneaky)
}

//Method one: If the requesting node has requested something before,
pub fn delay_to_avoid_surfers(prior_comms: u16) -> u64 {
    if prior_comms > 0 {
        return CONGESTION_DELAY_MS;
    } else {
        return 0;
    }
}

pub fn update_nodes(
    mut current_node: node::Node,
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
) -> std::io::Result<u16> {
    current_node.prior_communications += 1;
    let nodes_rwlock = nodes_arc.clone();
    let mut nodes_ptr = match nodes_rwlock.write() {
        Ok(ptr) => ptr,
        Err(e) => {
            info!("{}", e);
            return Err(Error::new(ErrorKind::Other, "well whatever"));
        }
    };
    info!("No problem re-adding the current node with an updated prior_comms");
    nodes_ptr.retain(|k| {
        ip_port_string(k.ip, k.port) != ip_port_string(current_node.ip, current_node.port)
    });
    let prior_node_comms = current_node.prior_communications;
    nodes_ptr.insert(current_node);
    Ok(prior_node_comms)
}

pub fn check_clients(
    ip: Ipv4Addr,
    port: u16,
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
) -> (bool, u16) {
    let stream_addr = ip_port_string(ip, port);
    info!("Accepted Client: {}", &stream_addr);
    let (current_node, was_sneaky) = node_of_packet(nodes_arc.clone(), &stream_addr);
    let prior_comms = update_nodes(current_node, nodes_arc.clone()).unwrap();
    (was_sneaky, prior_comms)
}

pub fn ip_port_string(ip: Ipv4Addr, port: u16) -> String {
    format!("{}:{}", ip, port)
}

pub fn update_client_number(increment: bool) {
    let mut current_clients_ptr = CURRENT_DATA_CLIENTS.write().unwrap();
    if increment {
        *current_clients_ptr += 1;
    } else {
        *current_clients_ptr -= 1;
    }
}

pub fn bind_udp_socket(mut port: u16) -> UdpSocket {
    loop {
        let udp_server_addr = ip_port_string(LOCALHOST, port);
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
