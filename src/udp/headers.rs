use std::fmt;

#[derive(PartialEq, Eq, Debug)]
pub enum PacketHeader {
    Disc,
    GET,
    GETACK,
    TCPReceiverExistence,
    UDPReceiverExistence,
    StopWaitData,
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
        "SR"
    }
    pub const fn tcp_get() -> &'static str {
        "TCPGET"
    }

    // TODO: Check each packet header for UDP, only the first packet for TCP.
    pub fn packet_type(packet_str: &str) -> PacketHeader {
        // Doing this repetitive work because the following PR has not been merged as of today:
        // https://github.com/rust-lang/rfcs/pull/2920
        const DISCOVERY: &'static str = PacketHeader::discovery();
        const GET: &'static str = PacketHeader::get();
        const ACK: &'static str = PacketHeader::ack();
        const TCP_GET: &'static str = PacketHeader::tcp_get();
        const STOP_AND_WAIT_DATA: &'static str = PacketHeader::stop_and_wait_data();
        const STOP_AND_WAIT_ACK: &'static str = PacketHeader::stop_and_wait_ack();
        const STOP_AND_WAIT_NAK: &'static str = PacketHeader::stop_and_wait_nak();
        const GO_BACK_N: &'static str = PacketHeader::go_back_n();
        const SELECTIVE_REPEAT: &'static str = PacketHeader::selective_repeat();
        let header_str = packet_str.lines().next().unwrap_or("");
        let header = [header_str, "\n"].join("");
        if header.starts_with(DISCOVERY) {
            PacketHeader::Disc
        } else if header.starts_with(GET) {
            PacketHeader::GET
        } else if header.starts_with(ACK) {
            PacketHeader::GETACK
        } else if header.starts_with(TCP_GET) {
            PacketHeader::TCPReceiverExistence
        } else if header.starts_with(STOP_AND_WAIT_DATA) {
            PacketHeader::StopWaitData
        } else if header.starts_with(STOP_AND_WAIT_ACK) {
            PacketHeader::StopWaitACK
        } else if header.starts_with(STOP_AND_WAIT_NAK) {
            PacketHeader::StopWaitNAK
        } else if header.starts_with(GO_BACK_N) {
            PacketHeader::GoBackN
        } else if header.starts_with(SELECTIVE_REPEAT) {
            PacketHeader::SRepeat
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
        } else if self == &PacketHeader::TCPReceiverExistence {
            display_str = PacketHeader::tcp_get();
        } else if self == &PacketHeader::StopWaitData {
            display_str = PacketHeader::stop_and_wait_data();
        } else if self == &PacketHeader::StopWaitACK {
            display_str = PacketHeader::stop_and_wait_ack();
        } else if self == &PacketHeader::StopWaitNAK {
            display_str = PacketHeader::stop_and_wait_nak();
        } else if self == &PacketHeader::GoBackN {
            display_str = PacketHeader::go_back_n();
        } else if self == &PacketHeader::SRepeat {
            display_str = PacketHeader::selective_repeat();
        } else {
            display_str = PacketHeader::discovery();
        }
        write!(f, "{}", display_str)
    }
}

pub struct DataHeader {
    pub conn_type: PacketHeader,
    pub udp_get_port: u16,
    pub file_name: String,
}

impl DataHeader {
    pub fn new(conn_type: PacketHeader, udp_get_port: u16, file_name: String) -> DataHeader {
        DataHeader {
            conn_type,
            udp_get_port,
            file_name,
        }
    }

    pub fn from_tcp_string(packet: String) -> DataHeader {
        let mut packet_lines = packet.lines();
        let udp_get_port = packet_lines.next().unwrap().parse::<u16>().unwrap_or(0);
        let file_name = packet_lines.next().unwrap_or("").to_string();
        let conn_type = PacketHeader::packet_type(&packet);
        DataHeader::new(conn_type, udp_get_port, file_name)
    }

    pub fn to_tcp_string(&self) -> String {
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
