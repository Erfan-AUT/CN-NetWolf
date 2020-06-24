use crate::udp::generate_address;
use crate::{BUF_SIZE, CURRENT_DATA_CLIENTS, LOCALHOST, STATIC_DIR};
use log::{info, warn};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
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

pub fn handle_client(stream: TcpStream, file_name: &str, delay: u64) -> std::io::Result<()> {
    let mut tcp_output_steam = BufWriter::new(stream);
    let file_addr = generate_file_address(file_name, false);
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

pub fn tcp_get_sender(
    incoming_ip_str: String,
    file_name: String,
    prior_comms: u16,
) -> std::io::Result<()> {
    let tcp_addr = generate_address(LOCALHOST, *crate::DATA_PORT);
    let listener = match TcpListener::bind(&tcp_addr) {
        Ok(lsner) => lsner,
        Err(_) => return Ok(()),
    };
    info!("Opened TCP Socket on: {}", tcp_addr);
    // Unly handles one client but whatever. :))
    for strm in listener.incoming() {
        let stream = strm?;
        // Make sure you're responding to the right client!
        let stream_ip = stream.local_addr()?.ip().to_string();
        info!("Stream IP is: {}", stream_ip);
        info!("Incoming address was: {}", incoming_ip_str);
        if stream_ip == incoming_ip_str {
            info!("Accepted Client");
            handle_client(stream, &file_name, delay_to_avoid_surfers(prior_comms))?;
            break;
        }
        warn!("Refused Client");
    }
    Ok(())
}
