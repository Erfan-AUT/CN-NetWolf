use crate::tcp::generate_file_address;
use crate::udp::{bind_udp_socket, generate_address};
use crate::LOCALHOST;
use std::collections::HashSet;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, RwLock};

pub fn sw_server(nodes_arc: Arc<RwLock<HashSet<node::Node>>>) -> std::io::Result<()> {
    let socket = bind_udp_socket(*crate::DATA_PORT);
    loop 
    Ok(())
}
