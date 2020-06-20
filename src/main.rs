use std::fs;
use std::net::Ipv4Addr;

// Make sure to read from an LF file!
const INIT_NODE_FILE: &str = "./nodes.txt";

#[derive(Debug)]
struct Node {
    name: String,
    ip: Ipv4Addr,
}

impl Node {
    fn new(name_str: &str, ip_str: &str) -> Node {
        let name_string = String::from(name_str);
        let ip_parsed = str_to_u8_vector(ip_str);
        Node {
            name: name_string,
            ip: Ipv4Addr::new(ip_parsed[0], ip_parsed[1], ip_parsed[2], ip_parsed[3]),
        }
    }
}

fn str_to_u8_vector(ip_str: &str) -> Vec<u8> {
    let mut ip_parsed: Vec<u8> = Vec::new();
    let split_by_dot = ip_str.split(".");
    for ip_part in split_by_dot {
        let ip_part_string = String::from(ip_part);
        let ip_part_u8 = ip_part_string.parse::<u8>().unwrap();
        ip_parsed.push(ip_part_u8);
    }
    ip_parsed
}

fn read_starting_nodes() -> Vec<Node> {
    let mut nodes: Vec<Node> = Vec::new();
    let data = fs::read_to_string(INIT_NODE_FILE).expect("Something's wrong with the file.");
    let split_by_line = data.split("\n");
    for line in split_by_line {
        let node_str: Vec<&str> = line.split(" ").collect();
        let node = Node::new(node_str[0], node_str[1]);
        nodes.push(node);
    }
    nodes
}

fn main() -> std::io::Result<()> {
    let _nodes = read_starting_nodes();
    println!("Hello, world!");
    Ok(())
}
