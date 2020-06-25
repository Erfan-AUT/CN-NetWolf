use crate::udp::{generate_address, bind_udp_socket};
use crate::LOCALHOST;
use crate::tcp::generate_file_address;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{UdpSocket, SocketAddr};

pub fn sw_sender(
    incoming_addr: SocketAddr,
    file_name: String,
    prior_comms: u16,
) -> std::io::Result<()> {
    let socket = bind_udp_socket(*crate::DATA_PORT);

    Ok(())
}
