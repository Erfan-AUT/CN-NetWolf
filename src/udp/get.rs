use rand::Rng;
use std::fmt;
use std::num::ParseIntError;

const PORT_MIN: u16 = 2000;
const PORT_MAX: u16 = 5000;

pub struct GETPair {
    pub file_name: String,
    pub tcp_port: u16,
}

impl fmt::Display for GETPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.file_name, self.tcp_port)
    }
}

impl GETPair {
    pub fn new(file_name: &str, tcp_port: u16) -> GETPair {
        let file_name = file_name.to_string();
        GETPair {
            file_name,
            tcp_port,
        }
    }

    pub fn from_str(input: &str) -> Result<GETPair, ParseIntError> {
        let pair_strs: Vec<&str> = input.split(" ").collect();
        Ok(GETPair {
            file_name: pair_strs[0].to_string(),
            tcp_port: pair_strs[1].parse()?,
        })
    }

    pub fn with_random_port(file_name: &str) -> GETPair {
        let file_name = file_name.to_string();
        let mut rng = rand::thread_rng();
        let tcp_port = rng.gen_range(PORT_MIN, PORT_MAX);
        GETPair {
            file_name,
            tcp_port
        }
    }

    pub fn random_get() -> GETPair {
        let mut rng = rand::thread_rng();
        let tcp_port = rng.gen_range(PORT_MIN, PORT_MAX);
        // null means that the node is presenting itself as a sender.
        let request_options = ["mamad.txt", "reza.mp4", "ahmad.png"];
        let option_index = rng.gen_range(0, request_options.len());
        let random_option = request_options[option_index];
        GETPair::new(random_option, tcp_port)
    }
}
