use crate::udp::generate_address;
use crate::LOCALHOST;
use crate::tcp::generate_file_address;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::UdpSocket;
pub fn sw_sender(
    incoming_ip_str: String,
    file_name: String,
    prior_comms: u16,
) -> std::io::Result<()> {
    let socket_addr = generate_address(LOCALHOST, *crate::DATA_PORT);
    Ok(())
}
