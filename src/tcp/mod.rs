use crate::dir::{file_list, generate_file_address};
use crate::networking::{
    check_clients, delay_to_avoid_surfers, ip_port_string, update_client_number, UDP_SERVER_PORT, LOCALHOST
};
use crate::node;
use crate::udp::headers::{PacketHeader, TCPHeader};
use log::{info, warn};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, RwLock};
use std::{thread, time};
use crate::networking::{self, BUF_SIZE};

// This function is not yet compliant with its corresponding TCP sender.
pub fn tcp_client(addr: SocketAddr, file_name: String) -> std::io::Result<()> {
    info!("Trying to connect to socket: {}", addr);
    let file_addr = generate_file_address(&file_name, true);
    let mut stream = TcpStream::connect(addr)?;
    let request_header = TCPHeader::new(PacketHeader::TCPGET, UDP_SERVER_PORT, file_name);
    stream.write(request_header.to_string().as_bytes()).unwrap();
    stream.shutdown(Shutdown::Write).unwrap();
    let mut tcp_input_stream = BufReader::new(stream);
    info!("Trying to create the receiving file for writing");
    let f = File::create(file_addr)?;
    let mut file_output_stream = BufWriter::new(f);
    info!("Starting to receive data from TCP socket");
    handle_both(&mut tcp_input_stream, &mut file_output_stream, 0)
}

pub fn handle_both<T: Read, U: Write>(
    input: &mut BufReader<T>,
    output: &mut BufWriter<U>,
    delay: u64,
) -> std::io::Result<()> {
    let mut buf = [0; BUF_SIZE];
    let mut size: usize = 1;
    let anti_surfing_interval = time::Duration::from_millis(delay);
    while size > 0 {
        size = input.read(&mut buf)?;
        output.write(&buf[..size])?;
        thread::sleep(anti_surfing_interval);
        info!("Read and Wrote {} bytes from/to sockets", size);
    }
    info!("Finished reading and writing!");
    Ok(())
}

pub fn handle_client(stream: TcpStream, file_name: String, delay: u64) -> std::io::Result<()> {
    let mut tcp_output_steam = BufWriter::new(stream);
    let file_addr = generate_file_address(&file_name, false);
    // let b = stream.local_addr();
    let f = File::open(file_addr)?;
    let mut file_input_stream = BufReader::new(f);
    update_client_number(true);
    let result = handle_both(&mut file_input_stream, &mut tcp_output_steam, delay);
    update_client_number(false);
    result
}

fn check_and_handle_clients(mut stream: TcpStream, nodes_arc: Arc<RwLock<HashSet<node::Node>>>) {
    let mut tcp_get_packet = String::new();
    stream.read_to_string(&mut tcp_get_packet).unwrap_or(0);
    let data_header = TCPHeader::from_string(tcp_get_packet);
    if data_header.conn_type == PacketHeader::TCPGET {
        // If old node, it's ok; if not, check again!
        let (was_sneaky, prior_comms) = check_clients(
            match stream.peer_addr().unwrap().ip() {
                IpAddr::V4(v4) => v4,
                IpAddr::V6(_) => return,
            },
            data_header.udp_get_port,
            nodes_arc,
        );
        if !was_sneaky || file_list().iter().any(|x| x == &data_header.file_name) {
            std::thread::spawn(move || {
                handle_client(
                    stream,
                    data_header.file_name,
                    delay_to_avoid_surfers(prior_comms),
                )
            });
        }
    } else {
        // Malicious packets BTFO
        warn!("Refused malicious Client");
        drop(stream);
    }
}

// First packet of every stream: Who you are and what you want (again)
// Because all sending is done through this one TCP Listener.
pub fn tcp_server(nodes_arc: Arc<RwLock<HashSet<node::Node>>>) -> std::io::Result<()> {
    let tcp_addr = ip_port_string(LOCALHOST, *networking::DATA_PORT);
    let listener = match TcpListener::bind(&tcp_addr) {
        Ok(lsner) => lsner,
        Err(_) => return Ok(()),
    };
    info!("Opened TCP Socket on: {}", tcp_addr);
    for stream in listener.incoming() {
        check_and_handle_clients(stream?, nodes_arc.clone());
    }
    Ok(())
}
