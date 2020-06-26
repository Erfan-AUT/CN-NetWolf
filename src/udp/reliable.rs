use crate::dir::{file_list, generate_file_address};
use crate::networking::{bind_udp_socket, node_of_packet};
use crate::networking::{ 
    self, check_clients, delay_to_avoid_surfers, ip_port_string, update_client_number, update_nodes, LOCALHOST, BUF_SIZE
};
use crate::node;
use crate::udp::headers::{PacketHeader, StopAndWaitHeader};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read, Error, ErrorKind};
use std::net::{SocketAddr, UdpSocket, Ipv4Addr, IpAddr};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::vec;
use std::{thread, time};

pub fn sw_server(nodes_arc: Arc<RwLock<HashSet<node::Node>>>) -> std::io::Result<()> {
    let socket = bind_udp_socket(*networking::DATA_PORT);
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
            IpAddr::V6(_) => return Ok(())
        };
        let rdt_port = addr.port();
        let rdt_address = ip_port_string(header_ip, rdt_port);
        if header.header_type == PacketHeader::RDTGET {
            let (was_sneaky, prior_comms) = check_clients(header_ip, header.get_port, nodes_arc.clone());
            if !was_sneaky || file_list().iter().any(|x| x == &header.file_name) {
                if !nodes_channels.contains_key(&rdt_address) {
                    let (sender, receiver) = mpsc::channel::<(StopAndWaitHeader, Vec<u8>)>();
                    nodes_channels.insert(rdt_address.clone(), sender);
                    // Spawn handle_client hanlder if it's a new node.
                    let new_socket = socket.try_clone().unwrap();
                    std::thread::spawn(move || sw_sender(new_socket, receiver, prior_comms));
                }
                let sender = match nodes_channels.get(&rdt_address) {
                    Some(snd) => snd,
                    None => continue,
                };
                sender.send((header, data.to_vec())).unwrap();
            }
        } else {
            continue;
        }
    }
    Ok(())
}

pub fn sw_sender(
    socket: UdpSocket,
    receiver: Receiver<(StopAndWaitHeader, Vec<u8>)>,
    prior_comms: u16,
) -> std::io::Result<()> {
    // Here, data is not important because we're the sender.
    let (header, _) = receiver.recv().unwrap();
    let file_name = header.file_name;
    let f = File::open(&file_name)?;
    let mut file_input_stream = BufReader::new(f);
    let header_ip = match header.ip {
        IpAddr::V4(v4) => v4,
        // Cause who tf is using Ipv6 with this?
        IpAddr::V6(_) => return Ok(())
    };
    let get_addr = ip_port_string(header_ip, header.get_port);
    let rdt_addr = ip_port_string(header_ip, header.rdt_port);
    let mut buf = [0; BUF_SIZE];
    let mut corrupt_packet_count = 0;
    loop {
        let (header, _) = receiver.recv().unwrap();
        let delay = delay_to_avoid_surfers(prior_comms);
        let anti_surfing_interval = time::Duration::from_millis(delay);
        thread::sleep(anti_surfing_interval);
        let header_ip = match header.ip {
            IpAddr::V4(v4) => v4,
            // Cause who tf is using Ipv6 with this?
            IpAddr::V6(_) => return Ok(())
        };
        let new_get_addr = ip_port_string(header_ip, header.get_port);
        let new_rdt_addr = ip_port_string(header_ip, header.rdt_port);
        if new_get_addr == get_addr && new_rdt_addr == rdt_addr && header.file_name == file_name {
            if header.header_type == PacketHeader::StopWaitACK {
                buf = [0; BUF_SIZE];
                let size = file_input_stream.read(&mut buf)?;
                if size > 0 {
                    socket.send_to(&buf, &rdt_addr).unwrap();
                }
            } else if header.header_type == PacketHeader::StopWaitNAK {
                socket.send_to(&buf, &rdt_addr).unwrap();
            } else {
                info!("Something has gone wrong with the packet's header");
                corrupt_packet_count += 1;
            }
            if corrupt_packet_count > 5 {
                warn!("Too many faulty packets");
                // Well not really ok but it's run in a separate thread, so who cares?
                return Ok(())
            }
        }
    }
}
