use rand::Rng;
use std::fmt;

pub struct GETRequest {
    pub file_name: String,
}

impl fmt::Display for GETRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.file_name)
    }
}

impl GETRequest {
    pub fn new(file_name: &str) -> GETRequest {
        let file_name = file_name.to_string();
        GETRequest { file_name }
    }
    pub fn random_get() -> GETRequest {
        let request_options = ["mamad.txt", "reza.mp4", "ahmad.png"];
        let mut rng = rand::thread_rng();
        let option_index = rng.gen_range(0, request_options.len());
        let random_option = request_options[option_index];
        GETRequest::new(random_option)
    }
}

pub struct GETResponse {
    pub tcp_port: i32,
}

impl fmt::Display for GETResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.tcp_port)
    }
}

impl GETResponse {
    pub fn new(tcp_port: i32) -> GETResponse {
        GETResponse { tcp_port }
    }
}
