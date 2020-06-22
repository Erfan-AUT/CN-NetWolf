use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::{fmt, fs};
// Make sure to read from an LF file!
const INIT_NODE_FILE: &str = "./nodes.txt";

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct Node {
    pub name: String,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub prior_communications: u16,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {}", self.name, self.ip, self.port)
    }
}

impl Default for Node {
    fn default() -> Node {
        Node {
            name: String::new(),
            ip: Ipv4Addr::new(0, 0, 0, 0),
            port: 0,
            prior_communications: 0,
        }
    }
}

impl Node {
    pub fn new(name_str: &str, ip_str: &str, port: u16) -> Node {
        let ip_parsed = str_to_u8_vector(ip_str);
        Node {
            name: String::from(name_str),
            ip: Ipv4Addr::new(ip_parsed[0], ip_parsed[1], ip_parsed[2], ip_parsed[3]),
            port,
            ..Default::default()
        }
    }

    pub fn nodes_to_string(nodes: &HashSet<Node>) -> String {
        let mut nodes_string = String::from("");
        for node in nodes {
            let a = node.to_string();
            nodes_string.push_str(&a);
            // For distinction in deserializing
            nodes_string.push('\n');
        }
        nodes_string.truncate(nodes_string.trim_end().len());
        nodes_string
    }
    pub fn multiple_from_string(data: String) -> HashSet<Node> {
        let mut nodes: HashSet<Node> = HashSet::new();
        let split_by_line = data.split("\n");
        for line in split_by_line {
            let node_strs: Vec<&str> = line.split(" ").collect();
            println!("{:?}", node_strs);
            // #communications with this new node is zero!
            let node = Node::new(
                node_strs[0],
                node_strs[1],
                node_strs[2].parse::<u16>().unwrap(),
            );
            nodes.insert(node);
        }
        nodes
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

pub fn read_starting_nodes() -> HashSet<Node> {
    let data = fs::read_to_string(INIT_NODE_FILE).expect("Something's wrong with the file.");
    return Node::multiple_from_string(data);
}
