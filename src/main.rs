// This feature has not been stabilized yet. For more info refer to:
// https://github.com/rust-lang/rust/issues/57563
// #![feature(const_fn)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate simple_logger;
mod dir;
mod node;
mod tcp;
mod udp;
use clap::{App, Arg};
mod networking;

use std::sync::{mpsc, RwLock};
use std::{env, io};

lazy_static! {
    static ref STATIC_DIR: RwLock<String> = RwLock::new(String::new());
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
        .arg(
            Arg::with_name("verbose")
                .short('v')
                .long("verbose")
                .takes_value(false)
                .about("Enables the program to be run in verbose mode."),
        )
        .get_matches();
    let init_nodes_dir = matches.value_of("list").unwrap_or("nodes.txt");
    let static_dir = matches.value_of("dir").unwrap_or("./static/").to_string();
    let is_verbose = matches.is_present("verbose");
    if is_verbose {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }
    *STATIC_DIR.write().unwrap() = static_dir;
    let (stdin_tx, stdin_rx) = mpsc::channel::<String>();
    let init_dir_string = init_nodes_dir.to_string();
    std::thread::spawn(move || udp::main_server(init_dir_string, stdin_rx));
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input != "quit" {
            stdin_tx.send(input.clone()).unwrap();
        } else {
            break;
        }
    }
    Ok(())
}
