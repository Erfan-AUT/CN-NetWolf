use crate::udp::{generate_address, get::GETPair};
use crate::{BUF_SIZE, STATIC_DIR, LOCALHOST};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};

fn generate_file_address(file_name: &str, sr: bool) -> String {
    let ptr_string = &*STATIC_DIR.lock().unwrap();
    let mut file_addr = String::from(ptr_string);
    file_addr.push_str(file_name);
    if sr {
        file_addr.push_str("-1.txt");
    }
    file_addr
}

pub async fn tcp_get_receiver((ip, res): (String, GETPair)) -> std::io::Result<()> {
    let stream = TcpStream::connect(generate_address(&ip, res.tcp_port))?;
    let mut tcp_input_stream = BufReader::new(stream);
    let file_addr = generate_file_address(&res.file_name, true);
    let f = File::open(file_addr)?;
    let mut file_output_stream = BufWriter::new(f);
    handle_both(&mut tcp_input_stream, &mut file_output_stream)
}

pub fn handle_both<T: Read, U: Write>(
    input: &mut BufReader<T>,
    output: &mut BufWriter<U>,
) -> std::io::Result<()> {
    let mut buf = [0; BUF_SIZE];
    let mut size: usize = 1;
    while size > 0 {
        size = input.read(&mut buf)?;
        output.write(&buf)?;
    }
    Ok(())
}

pub fn handle_client(stream: TcpStream, file_name: &str) -> std::io::Result<()> {
    let mut tcp_output_steam = BufWriter::new(stream);
    let file_addr = generate_file_address(file_name, false);
    let f = File::open(file_addr)?;
    let mut file_input_stream = BufReader::new(f);
    handle_both(&mut file_input_stream, &mut tcp_output_steam)
}

pub async fn tcp_get_sender(starting_point: GETPair) -> std::io::Result<()> {
    let tcp_addr = generate_address(LOCALHOST, starting_point.tcp_port);
    let listener = match TcpListener::bind(tcp_addr) {
        Ok(lsner) => lsner,
        Err(_) => return Ok(()),
    };
    // Basically only handles one client but whatever. :))
    for stream in listener.incoming() {
        handle_client(stream?, &starting_point.file_name)?;
    }
    Ok(())
}
