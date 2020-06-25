#[derive(PartialEq, Debug)]
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

    pub fn packet_type(packet_str: &str) -> PacketHeader {
        // Doing this repetitive work because the following PR has not been merged as of today:
        // https://github.com/rust-lang/rfcs/pull/2920
        const DISCOVERY: &'static str = PacketHeader::discovery();
        const GET: &'static str = PacketHeader::get();
        const ACK: &'static str = PacketHeader::ack();
        let header_str = packet_str.lines().next().unwrap_or("");
        let header = [header_str, "\n"].join("");
        if header.starts_with(DISCOVERY) {
            PacketHeader::Disc
        } else if header.starts_with(GET) {
            PacketHeader::GET
        } else if header.starts_with(ACK) {
            PacketHeader::GETACK
        } else {
            PacketHeader::Unrecognized
        }
    }

    // Check each packet header for UDP, only the first packet for TCP.
    pub fn udp_transfer_packet_type(packet_str: &str) {
        const STOP_AND_WAIT: &'static str = PacketHeader::stop_and_wait_data();
        const GO_BACK_N: &'static str = PacketHeader::go_back_n();
        const SELECTIVE_REPEAT: &'static str = PacketHeader::selective_repeat();
    }

    pub fn tcp_transfer_packet_type(packet_str: &str) -> PacketHeader {
        if packet_str.starts_with(PacketHeader::tcp_get()) {
            PacketHeader::TCPReceiverExistence
        } else {
            PacketHeader::Unrecognized
        }
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
