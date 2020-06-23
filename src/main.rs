#[macro_use]
extern crate lazy_static;
mod dir;
mod node;
mod tcp;
mod udp;
use clap::{App, Arg};
use rand::Rng;
use std::sync::{mpsc, RwLock};
use std::{env, io};

// pub const STATIC_DIR: &'static str = "./static/";
pub const BUF_SIZE: usize = 8192;
pub const PORT_MIN: u16 = 2000;
pub const PORT_MAX: u16 = 5000;
pub const LOCALHOST: &str = "127.0.0.1";
pub const MAX_TCP_CLIENTS: u16 = 3;

lazy_static! {
    static ref TCP_PORT: u16 = random_tcp_port();
    static ref STATIC_DIR: RwLock<String> = RwLock::new(String::new());
    static ref CURRENT_TCP_CLIENTS: RwLock<u16> = RwLock::new(0);
}

fn random_tcp_port() -> u16 {
    let mut r = rand::thread_rng();
    r.gen_range(PORT_MIN, PORT_MAX)
}

fn main() -> std::io::Result<()> {
    env::set_var("RUST_BACKTRACE", "full");
    let matches = App::new("Netwolf?")
        .version("BROTHER")
        .author("Conan O'Brien <conan@teamcoco.com>")
        .about("Has an IQ of 160")
        .arg(
            Arg::with_name("list")
                .short('l')
                .long("list")
                .takes_value(true)
                .about("A file containing the list of this node's initial known nodes"),
        )
        .arg(
            Arg::with_name("dir")
                .short('d')
                .long("directory")
                .takes_value(true)
                .about("The directory whose files this node is going to share."),
        )
        .get_matches();
    let init_nodes_dir = matches.value_of("list").unwrap_or("nodes.txt");
    let static_dir = matches.value_of("dir").unwrap_or("./static/").to_string();
    *STATIC_DIR.write().unwrap() = static_dir;
    let (stdin_tx, stdin_rx) = mpsc::channel::<String>();
    let init_dir_string = init_nodes_dir.to_string();
    std::thread::spawn(move || udp::udp_server(init_dir_string, stdin_rx));
    let mut input = String::new();
    loop {
        io::stdin().read_line(&mut input)?;
        if input != "quit" {
            stdin_tx.send(input.clone()).unwrap();
        } else {
            break;
        }
    }
    Ok(())
}
