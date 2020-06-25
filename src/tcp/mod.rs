use crate::dir::file_list;
use crate::node;
use crate::udp::{generate_address, headers::PacketHeader, node_of_packet};
use crate::{BUF_SIZE, CURRENT_DATA_CLIENTS, LOCALHOST, MAX_DATA_CLIENTS, STATIC_DIR};
use log::{info, warn};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::{thread, time};

const CONGESTION_DELAY_MS: u64 = 500;

// To avoid over-writing already existing files.
pub fn generate_file_address(file_name: &str, sr: bool) -> String {
    let static_dir = &*STATIC_DIR.read().unwrap();
    let buf_immut = PathBuf::new().join(static_dir).join(file_name);
    let mut display_str = String::from(buf_immut.to_str().unwrap());
    // This stupid duplication is the only way I could get away with
    // cloning a Path. JESUS 'EFFIN CHRIST
    if sr {
        let mut file_path_buf = PathBuf::new().join(static_dir).join(file_name);
        let file_extension = buf_immut.extension().unwrap_or(OsStr::new(".txt"));
        file_path_buf.set_extension("");
        file_path_buf.push("-1");
        file_path_buf.push(file_extension);
        display_str = String::from(Path::new(&file_path_buf).to_str().unwrap());
    }
    // let file_path = file_path_buf.as_path();
    info!("Destination for the incoming file is: {}", &display_str);
    display_str
}

// This function is not yet compliant with its corresponding TCP sender.
pub fn tcp_get_receiver(addr: SocketAddr, file_name: String) -> std::io::Result<()> {
    info!("Trying to connect to socket: {}", addr);
    let stream = TcpStream::connect(addr)?;
    let mut tcp_input_stream = BufReader::new(stream);
    let file_addr = generate_file_address(&file_name, true);
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
    let discovery_interval = time::Duration::from_millis(delay);
    while size > 0 {
        size = input.read(&mut buf)?;
        output.write(&buf[..size])?;
        thread::sleep(discovery_interval);
        info!("Read and Wrote {} bytes from/to sockets", size);
    }
    info!("Finished reading and writing!");
    Ok(())
}

fn update_client_number(increment: bool) {
    let mut current_clients_ptr = CURRENT_DATA_CLIENTS.write().unwrap();
    if increment {
        *current_clients_ptr += 1;
    } else {
        *current_clients_ptr -= 1;
    }
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
        &generate_address(&k.ip.to_string(), k.port)
            != &generate_address(&current_node.ip.to_string(), current_node.port)
    });
    let prior_node_comms = current_node.prior_communications;
    nodes_ptr.insert(current_node);
    Ok(prior_node_comms)
}

fn check_and_handle_clients(
    mut stream: TcpStream,
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
    sneaky_arc: Arc<RwLock<u16>>,
) {
    let mut tcp_get_packet = String::new();
    stream.read_to_string(&mut tcp_get_packet).unwrap_or(0);
    let mut packet_lines = tcp_get_packet.lines();
    let tcp_get_header = packet_lines.next().unwrap_or("");
    if PacketHeader::tcp_transfer_packet_type(tcp_get_header) == PacketHeader::TCPReceiverExistence
    {
        let stream_ip = stream.peer_addr().unwrap().ip();
        let udp_get_port = packet_lines.next().unwrap().parse::<u16>().unwrap();
        let stream_addr = node::Node::ip_port_string(stream_ip, udp_get_port);
        let (current_node, was_sneaky) = node_of_packet(
            nodes_arc.clone(),
            sneaky_arc.clone(),
            &stream_addr,
        );
        let prior_comms = match update_nodes(current_node, nodes_arc.clone()) {
            Ok(comms) => comms,
            Err(_) => return,
        };
        let file_name = packet_lines.next().unwrap().to_string();
        // If old node, it's ok; if not, check again!
        if !was_sneaky || file_list().iter().any(|x| x == &file_name) {
            let mut _client_count = *CURRENT_DATA_CLIENTS.write().unwrap();
            _client_count += 1;
            info!("Accepted Client: {}", &stream_addr);
            std::thread::spawn(move || {
                handle_client(stream, file_name, delay_to_avoid_surfers(prior_comms))
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
pub fn tcp_server(
    nodes_arc: Arc<RwLock<HashSet<node::Node>>>,
    sneaky_arc: Arc<RwLock<u16>>,
) -> std::io::Result<()> {
    let tcp_addr = generate_address(LOCALHOST, *crate::DATA_PORT);
    let listener = match TcpListener::bind(&tcp_addr) {
        Ok(lsner) => lsner,
        Err(_) => return Ok(()),
    };
    info!("Opened TCP Socket on: {}", tcp_addr);
    for stream in listener.incoming() {
        check_and_handle_clients(stream?, nodes_arc.clone(), sneaky_arc.clone());
    }
    Ok(())
}
