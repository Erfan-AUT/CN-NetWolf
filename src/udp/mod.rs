use std::vec;
use std::net::UdpSocket;
use crate::node;

const UDP_SERVER_PORT: i32 = 3321;
const LOCALHOST: &str = "127.0.0.1:";
const BUF_SIZE: usize = 8192;

fn generate_socket() -> UdpSocket {
    let mut current_server_port = UDP_SERVER_PORT;
    loop {
        let udp_server_addr = server_address(LOCALHOST, current_server_port);
        let _try_socket = match UdpSocket::bind(udp_server_addr) {
            Ok(sckt) => return sckt,
            Err(_) => ()
        };
        current_server_port += 1;
    }
}

fn server_address(ip: &str, port:i32) -> String {
    let mut addr = String::from(ip);
    addr.push_str(":");
    addr.push_str(&port.to_string());
    addr
}

fn reverse_udp(nodes: vec::Vec<node::Node>) -> std::io::Result<()> {
    let socket = generate_socket();
    for node in nodes {
        
    }
    let mut buf = [0; BUF_SIZE];
    let (amt, src) = socket.recv_from(&mut buf)?;
    println!("{}", String::from_utf8_lossy(&buf));
    let buf = &mut buf[..amt];
    buf.reverse();
    socket.send_to(buf, &src)?;
    Ok(())
}


fn udp_server() {

}