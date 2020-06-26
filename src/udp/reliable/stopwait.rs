use crate::dir::{file_list, generate_file_address};
use crate::networking::{
    self, check_clients, delay_to_avoid_surfers, ip_port_string, update_client_number,
    update_nodes, BUF_SIZE, DATA_RECEIVER_PORT, LOCALHOST, UDP_GET_PORT,
};
use crate::networking::{bind_udp_socket, node_of_packet};
use crate::node;
use crate::udp::headers::{PacketHeader, StopAndWaitHeader, RDT_HEADER_SIZE};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter, Error, ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::vec;
use std::{thread, time};

pub fn sw_server(nodes_arc: Arc<RwLock<HashSet<node::Node>>>) -> std::io::Result<()> {
    let socket = bind_udp_socket(*networking::DATA_SENDER_PORT, false);
    let mut nodes_channels: HashMap<String, Sender<(StopAndWaitHeader, Vec<u8>)>> = HashMap::new();
    let mut buf = [0; BUF_SIZE];
    loop {
        // This function is the only one reading from the socket!
        let (_, addr) = socket.recv_from(&mut buf).unwrap();
        let buf_clone = buf.clone();
        let (header, data) = StopAndWaitHeader::from_bytes(&buf_clone, addr.ip());
        let header_ip = match header.ip {
            IpAddr::V4(v4) => v4,
            // Cause who tf is using Ipv6 with this?
            IpAddr::V6(_) => return Ok(()),
        };
        let client_rdt_port = addr.port();
        let client_rdt_address = ip_port_string(header_ip, client_rdt_port);
        if header.header_type == PacketHeader::RDTGET {
            info!("Received S&W GET packet");
            let (was_sneaky, prior_comms) =
                check_clients(header_ip, header.get_port, nodes_arc.clone());
            if !was_sneaky || file_list().iter().any(|x| x == &header.file_name) {
                let (sender, receiver) = mpsc::channel::<(StopAndWaitHeader, Vec<u8>)>();
                nodes_channels.insert(client_rdt_address.clone(), sender.clone());
                // Spawn client_handler if it's a new node.
                let new_socket = socket.try_clone().unwrap();
                info!(
                    "Spawning a new sender thread for socket: {}",
                    client_rdt_address
                );
                std::thread::spawn(move || sw_sender(new_socket, receiver, prior_comms));
                sender.send((header, data.to_vec())).unwrap();
            }
        } else if header.header_type == PacketHeader::StopWaitACK
            || header.header_type == PacketHeader::StopWaitNAK
        {
            let sender = match nodes_channels.get(&client_rdt_address) {
                Some(snd) => snd,
                None => continue,
            };
            sender.send((header, data.to_vec())).unwrap();
        }
    }
}

pub fn read_and_write(
    input: &mut BufReader<File>,
    buf: &mut [u8],
    socket: &UdpSocket,
    rdt_addr: &str,
) -> std::io::Result<usize> {
    info!("Sending file data through socket!");
    let size = input.read(buf)?;
    // 0 means that we have reached EOF.
    if size == 0 {
        info!("Finished reading and writing!");
        socket
            .send_to(PacketHeader::rdt_end().as_bytes(), &rdt_addr)
            .unwrap();
        return Ok(0);
    } else {
        socket.send_to(&buf[..size], rdt_addr).unwrap();
    }
    Ok(size)
}

pub fn sw_sender(
    socket: UdpSocket,
    receiver: Receiver<(StopAndWaitHeader, Vec<u8>)>,
    prior_comms: u16,
) -> std::io::Result<()> {
    // Here, data is not important because we're the sender.
    let (header, _) = receiver.recv().unwrap();
    info!("Received data from channel (as it should)");
    let file_name = header.file_name;
    let file_addr = generate_file_address(&file_name, false);
    let f = File::open(&file_addr)?;
    let mut file_input_stream = BufReader::new(f);
    let header_ip = match header.ip {
        IpAddr::V4(v4) => v4,
        // Cause who tf is using Ipv6 with this?
        IpAddr::V6(_) => return Ok(()),
    };
    let get_addr = ip_port_string(header_ip, header.get_port);
    let rdt_addr = ip_port_string(header_ip, header.rdt_port);
    let mut buf = [0; BUF_SIZE];
    let mut corrupt_packet_count = 0;
    let mut size: usize =
        read_and_write(&mut file_input_stream, &mut buf, &socket, &rdt_addr).unwrap();
    if size > 0 {
        loop {
            info!("Waiting for client response");
            let (header, _) = receiver.recv().unwrap();
            // Received new client response
            let delay = delay_to_avoid_surfers(prior_comms);
            let anti_surfing_interval = time::Duration::from_millis(delay);
            thread::sleep(anti_surfing_interval);
            info!("Finished sleeping");
            let header_ip = match header.ip {
                IpAddr::V4(v4) => v4,
                // Cause who tf is using Ipv6 with this?
                IpAddr::V6(_) => return Ok(()),
            };
            let new_get_addr = ip_port_string(header_ip, header.get_port);
            let new_rdt_addr = ip_port_string(header_ip, header.rdt_port);
            if new_get_addr == get_addr && new_rdt_addr == rdt_addr && header.file_name == file_name
            {
                if header.header_type == PacketHeader::StopWaitACK {
                    info!("Received ACK");
                    size = read_and_write(&mut file_input_stream, &mut buf, &socket, &rdt_addr)
                        .unwrap();
                    if size == 0 {
                        return Ok(());
                    }
                    socket.send_to(&buf[..size], &rdt_addr).unwrap();
                } else if header.header_type == PacketHeader::StopWaitNAK {
                    info!("Received NAK");
                    socket.send_to(&buf[..size], &rdt_addr).unwrap();
                } else {
                    info!("Something has gone wrong with the packet's header");
                    corrupt_packet_count += 1;
                }
                if corrupt_packet_count > 5 {
                    warn!("Too many faulty packets");
                    // Well not really ok but it's run in a separate thread, so who cares?
                    return Ok(());
                }
            } else {
                info!("Conditions were not satisfied");
            }
        }
    }
    Ok(())
}

pub fn three_headers(
    get_port: u16,
    rdt_port: u16,
    file_name: &str,
    ip: IpAddr,
) -> (StopAndWaitHeader, StopAndWaitHeader, StopAndWaitHeader) {
    let get_header =
        StopAndWaitHeader::new(PacketHeader::RDTGET, get_port, rdt_port, file_name, ip);
    let ack_header =
        StopAndWaitHeader::new(PacketHeader::StopWaitACK, get_port, rdt_port, file_name, ip);
    let nak_header =
        StopAndWaitHeader::new(PacketHeader::StopWaitNAK, get_port, rdt_port, file_name, ip);
    (get_header, ack_header, nak_header)
}

pub fn sw_client(sender_addr: SocketAddr, file_name: String) -> std::io::Result<()> {
    info!("Trying to connect to S&W Data Socket: {}", sender_addr);
    let file_addr = generate_file_address(&file_name, true);
    let localhost = IpAddr::V4(LOCALHOST);
    let recv_addr = SocketAddr::new(localhost, *DATA_RECEIVER_PORT);
    let socket = UdpSocket::bind(recv_addr).unwrap();
    let f = File::create(file_addr).unwrap();
    let mut file_output_stream = BufWriter::new(f);
    // Making the UDP connection "duplex".
    socket.connect(sender_addr).unwrap();
    let timeout: Duration = Duration::new(3, 0);
    socket.set_write_timeout(Some(timeout)).unwrap();
    socket.set_read_timeout(Some(timeout)).unwrap();
    let (get_header, ack_header, nak_header) =
        three_headers(UDP_GET_PORT, *DATA_RECEIVER_PORT, &file_name, localhost);
    // Send data GET packet
    let failure_addr = SocketAddr::new(localhost, 0);
    info!("The udp get packet is: {}", get_header.as_string());
    info!(
        "Packet size in bytes: {}",
        get_header.as_string().as_bytes().len()
    );
    let get_header_vec = get_header.as_vec();
    let get_header_slice = get_header_vec.as_slice();
    let ack_header_vec = ack_header.as_vec();
    let ack_header_slice = ack_header_vec.as_slice();
    let nak_header_vec = nak_header.as_vec();
    let nak_header_slice = nak_header_vec.as_slice();
    socket.send(get_header_slice).unwrap();
    loop {
        let mut buf = [0; BUF_SIZE];
        // No malicious packet can come through because we've connected it to one target!
        let (size, data_addr) = match socket.recv_from(&mut buf) {
            Ok((size, addr)) => (size, addr),
            Err(_) => (0, failure_addr),
        };
        info!("Read {} bytes from socket", size);
        if PacketHeader::is_end(&buf[..RDT_HEADER_SIZE as usize]) {
            info!("Received END packet");
            drop(file_output_stream);
            return Ok(());
        }
        if data_addr == sender_addr {
            info!("Received new data from server!");
            if size > 0 {
                info!("Sending ACK");
                socket.send(ack_header_slice).unwrap();
                let buf_clone = buf.clone();
                file_output_stream.write(&buf_clone[..size]).unwrap();
            }
        } else {
            info!("Sending NAK");
            info!("Data addr is: {}", data_addr);
            info!("Supposed addr is: {}", sender_addr);
            socket.send(nak_header_slice).unwrap();
        }
    }
    Ok(())
}
