use std::io::{BufReader, BufWriter, Read, Write};
use std::net::UdpSocket;
use crate::LOCALHOST;
use crate::udp::generate_address;
pub fn sw_sender(incoming_ip_str: String, file_name: String, prior_comms: u16) -> std::io::Result<()> {
    // Ah yes, the "TCP" address.
    let socket_addr = generate_address(LOCALHOST, *crate::TCP_PORT);
    Ok(())
}
