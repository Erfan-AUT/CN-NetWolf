use crate::udp::{generate_address, get::GETPair};
use crate::{BUF_SIZE, LOCALHOST, STATIC_DIR};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::{thread, time};

const CONGESTION_DELAY_MS: u64 = 500;
// const MAX_PRIOR: u16 = 3;

fn generate_file_address(file_name: &str, sr: bool) -> String {
    let ptr_string = &*STATIC_DIR.lock().unwrap();
    let mut file_addr = String::from(ptr_string);
    file_addr.push_str(file_name);
    if sr {
        file_addr.push_str("-1.txt");
    }
    file_addr
}

pub fn tcp_get_receiver(addr:SocketAddr, file_name: String) -> std::io::Result<()> {
    let stream = TcpStream::connect(addr)?;
    let mut tcp_input_stream = BufReader::new(stream);
    let file_addr = generate_file_address(&file_name, true);
    let f = File::open(file_addr)?;
    let mut file_output_stream = BufWriter::new(f);
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
        output.write(&buf)?;
        thread::sleep(discovery_interval);
    }
    Ok(())
}

pub fn handle_client(stream: TcpStream, file_name: &str, delay: u64) -> std::io::Result<()> {
    let mut tcp_output_steam = BufWriter::new(stream);
    let file_addr = generate_file_address(file_name, false);
    let f = File::open(file_addr)?;
    let mut file_input_stream = BufReader::new(f);
    handle_both(&mut file_input_stream, &mut tcp_output_steam, delay)
}

pub fn tcp_get_sender(
    incoming_addr: String,
    file_name: String,
    prior_comms: u16,
) -> std::io::Result<()> {
    let tcp_addr = generate_address(LOCALHOST, *crate::TCP_PORT);
    let mut delay: u64 = 0;
    let listener = match TcpListener::bind(tcp_addr) {
        Ok(lsner) => lsner,
        Err(_) => return Ok(()),
    };
    // Unly handles one client but whatever. :))
    for stream in listener.incoming() {
        let strm = stream?;
        // Make sure you're responding to the right client!
        if strm.local_addr()?.ip().to_string() == incoming_addr {
            if prior_comms > 0 {
                delay = CONGESTION_DELAY_MS;
            }
            handle_client(strm, &file_name, delay)?;
            break;
        }
    }
    Ok(())
}
