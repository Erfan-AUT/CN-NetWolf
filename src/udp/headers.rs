use std::fmt;
use std::mem::size_of;
use std::net::IpAddr;

const RDT_HEADER_SIZE: u16 = 3;

pub enum ConnectionType {
    TCP,
    SAndW,
    GoBackN,
    SRepeat,
}

impl Default for ConnectionType {
    fn default() -> ConnectionType {
        ConnectionType::TCP
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum PacketHeader {
    Disc,
    GET,
    GETACK,
    TCPGET,
    RDTGET,
    StopWaitACK,
    StopWaitNAK,
    GoBackN,
    SRepeat,
    Unrecognized,
}

impl PacketHeader {
    // So apparently const functions work without even enabling the feature!
    // These three have an \n in the front because they're going to be
    // Parsed as strings, but the others should be treated as raw bytes.
    pub const fn discovery() -> &'static str {
        "DISC\n"
    }
    pub const fn ack() -> &'static str {
        "ACK\n"
    }
    pub const fn get() -> &'static str {
        "GET\n"
    }
    pub const fn tcp_get() -> &'static str {
        "TCPGET"
    }

    // All of the following should be of size 3.
    pub const fn rdt_get() -> &'static str {
        "RDT"
    }
    pub const fn stop_and_wait_data() -> &'static str {
        "SWD"
    }
    pub const fn stop_and_wait_ack() -> &'static str {
        "SWA"
    }
    pub const fn stop_and_wait_nak() -> &'static str {
        "SWN"
    }
    pub const fn go_back_n() -> &'static str {
        "GBN"
    }
    pub const fn selective_repeat() -> &'static str {
        "SER"
    }

    // TODO: Check each packet header for UDP, only the first packet for TCP.
    pub fn packet_type(packet_str: &str) -> PacketHeader {
        // Doing this repetitive work because the following PR has not been merged as of today:
        // https://github.com/rust-lang/rfcs/pull/2920
        const DISCOVERY: &'static str = PacketHeader::discovery();
        const GET: &'static str = PacketHeader::get();
        const ACK: &'static str = PacketHeader::ack();
        const TCP_GET: &'static str = PacketHeader::tcp_get();
        const STOP_AND_WAIT_ACK: &'static str = PacketHeader::stop_and_wait_ack();
        const STOP_AND_WAIT_NAK: &'static str = PacketHeader::stop_and_wait_nak();
        const GO_BACK_N: &'static str = PacketHeader::go_back_n();
        const SELECTIVE_REPEAT: &'static str = PacketHeader::selective_repeat();
        const RDT: &'static str = PacketHeader::rdt_get();
        let header_str = packet_str.lines().next().unwrap_or("");
        let header = [header_str, "\n"].join("");
        if header.starts_with(DISCOVERY) {
            PacketHeader::Disc
        } else if header.starts_with(GET) {
            PacketHeader::GET
        } else if header.starts_with(ACK) {
            PacketHeader::GETACK
        } else if header.starts_with(TCP_GET) {
            PacketHeader::TCPGET
        } else if header.starts_with(STOP_AND_WAIT_ACK) {
            PacketHeader::StopWaitACK
        } else if header.starts_with(STOP_AND_WAIT_NAK) {
            PacketHeader::StopWaitNAK
        } else if header.starts_with(GO_BACK_N) {
            PacketHeader::GoBackN
        } else if header.starts_with(SELECTIVE_REPEAT) {
            PacketHeader::SRepeat
        } else if header.starts_with(RDT) {
            PacketHeader::RDTGET
        } else {
            PacketHeader::Unrecognized
        }
    }
}

impl fmt::Display for PacketHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut display_str: &'static str = "";
        if self == &PacketHeader::Disc {
            display_str = PacketHeader::discovery();
        } else if self == &PacketHeader::GET {
            display_str = PacketHeader::discovery();
        } else if self == &PacketHeader::GETACK {
            display_str = PacketHeader::ack();
        } else if self == &PacketHeader::TCPGET {
            display_str = PacketHeader::tcp_get();
        } else if self == &PacketHeader::StopWaitACK {
            display_str = PacketHeader::stop_and_wait_ack();
        } else if self == &PacketHeader::StopWaitNAK {
            display_str = PacketHeader::stop_and_wait_nak();
        } else if self == &PacketHeader::GoBackN {
            display_str = PacketHeader::go_back_n();
        } else if self == &PacketHeader::SRepeat {
            display_str = PacketHeader::selective_repeat();
        } else if self == &PacketHeader::RDTGET {
            display_str = PacketHeader::rdt_get();
        } else {
            display_str = PacketHeader::discovery();
        }
        write!(f, "{}", display_str)
    }
}

pub struct TCPHeader {
    pub conn_type: PacketHeader,
    pub udp_get_port: u16,
    pub file_name: String,
}

impl TCPHeader {
    pub fn new(conn_type: PacketHeader, udp_get_port: u16, file_name: String) -> TCPHeader {
        TCPHeader {
            conn_type,
            udp_get_port,
            file_name,
        }
    }

    pub fn from_string(packet: String) -> TCPHeader {
        let mut packet_lines = packet.lines();
        let packet_type = packet_lines.next().unwrap();
        let conn_type = PacketHeader::packet_type(&packet_type);
        let udp_get_port = packet_lines.next().unwrap().parse::<u16>().unwrap_or(0);
        let file_name = packet_lines.next().unwrap_or("").to_string();
        TCPHeader::new(conn_type, udp_get_port, file_name)
    }

    pub fn to_string(&self) -> String {
        format!(
            "{}\n{}\n{}",
            self.conn_type, self.udp_get_port, self.file_name
        )
    }
}

pub enum StdinHeader {
    LIST,
    GET,
}

impl StdinHeader {
    pub fn get() -> &'static str {
        "get"
    }
    pub fn list() -> &'static str {
        "list"
    }
}

pub struct StopAndWaitHeader {
    pub header_type: PacketHeader,
    pub header_size: u16,
    pub get_port: u16,
    pub rdt_port: u16,
    pub file_name: String,
    pub ip: IpAddr,
}

impl StopAndWaitHeader {
    pub fn new(
        header_type: PacketHeader,
        get_port: u16,
        rdt_port: u16,
        file_name: &str,
        ip: IpAddr,
    ) -> StopAndWaitHeader {
        StopAndWaitHeader {
            header_type,
            header_size: StopAndWaitHeader::find_header_size(&file_name),
            get_port,
            rdt_port,
            file_name: String::from(file_name),
            ip,
        }
    }

    fn u16_from_bytes(buf: &[u8]) -> u16 {
        let byte_str = std::str::from_utf8(buf).unwrap();
        byte_str.parse::<u16>().unwrap()
    }

    pub fn from_bytes(buf: &[u8], ip: IpAddr) -> (StopAndWaitHeader, &[u8]) {
        let header = PacketHeader::packet_type(
            std::str::from_utf8(&buf[..RDT_HEADER_SIZE as usize]).unwrap_or(""),
        );
        let size = size_of::<u16>();
        let header_size = StopAndWaitHeader::u16_from_bytes(&buf[..size]) as usize;
        let get_port = StopAndWaitHeader::u16_from_bytes(&buf[size..size * 2]);
        let rdt_port = StopAndWaitHeader::u16_from_bytes(&buf[size * 2..size * 3]);
        let file_name = std::str::from_utf8(&buf[size * 3..header_size])
            .unwrap()
            .to_string();
        (
            StopAndWaitHeader::new(header, get_port, rdt_port, &file_name, ip),
            &buf[header_size..],
        )
    }

    pub fn as_string(&self) -> String {
        let mut header_str = String::new();
        header_str.push_str(&self.header_type.to_string());
        header_str.push_str(&self.header_size.to_string());
        header_str.push_str(&self.get_port.to_string());
        header_str.push_str(&self.rdt_port.to_string());
        header_str.push_str(&self.file_name);
        header_str
    }

    pub fn find_header_size(file_name: &str) -> u16 {
        RDT_HEADER_SIZE + (size_of::<u16>() as u16) * 3 + file_name.as_bytes().len() as u16
    }

    pub fn as_vec (&self) -> Vec<u8> {
        let type_str = self.header_type.to_string();
        let type_bytes = type_str.as_bytes();
        let size_bytes = self.header_size.to_le_bytes();
        let get_port_bytes = self.get_port.to_le_bytes();
        let rdt_port_bytes = self.rdt_port.to_le_bytes();
        let file_name_bytes = self.file_name.as_bytes();
        [type_bytes, &size_bytes, &get_port_bytes, &rdt_port_bytes, file_name_bytes].concat()
    }
}
