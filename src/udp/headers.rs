

#[derive(PartialEq)]
pub enum PacketHeader {
    Disc,
    GET,
    GETACK,
    StopWait,
    GoBackN,
    SRepeat,
    Unrecognized,
}

impl PacketHeader {
    // So apparently const functions work without even enabling the feature!
    pub const fn discovery() -> &'static str {
        "DISC\n"
    }
    pub const fn ack() -> &'static str {
        "ACK\n"
    }
    pub const fn get() -> &'static str {
        "GET\n"
    }
    pub const fn stop_and_wait() -> &'static str {
        "SAW\n"
    }
    pub const fn go_back_n() -> &'static str {
        "GBN\n"
    }
    pub const fn selective_repeat() -> &'static str {
        "SR\n"
    }

    pub fn packet_type(header: &str) -> PacketHeader {
        // Doing this repetitive work because the following PR has not been merged as of today:
        // https://github.com/rust-lang/rfcs/pull/2920
        const DISCOVERY: &'static str = PacketHeader::discovery();
        const GET: &'static str = PacketHeader::get();
        const ACK: &'static str = PacketHeader::ack();
        const STOP_AND_WAIT: &'static str = PacketHeader::stop_and_wait();
        const GO_BACK_N: &'static str = PacketHeader::go_back_n();
        const SELECTIVE_REPEAT: &'static str = PacketHeader::selective_repeat();
        match header {
            DISCOVERY => PacketHeader::Disc,
            GET => PacketHeader::GET,
            ACK => PacketHeader::GETACK,
            STOP_AND_WAIT => PacketHeader::StopWait,
            GO_BACK_N => PacketHeader::GoBackN,
            SELECTIVE_REPEAT => PacketHeader::SRepeat,
            &_ => PacketHeader::Unrecognized
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
