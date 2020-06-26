use crate::networking;
use crate::udp::headers::PacketHeader;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::{fmt, fs};
// Make sure to read from an LF file!

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
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
        let mut nodes_string = String::from(PacketHeader::discovery());
        for node in nodes {
            let a = node.to_string();
            nodes_string.push_str(&a);
            // For distinction in deserializing
            nodes_string.push('\n');
        }
        nodes_string.truncate(nodes_string.trim_end().len());
        nodes_string
    }

    pub fn has_same_address(&self, other_str: &str) -> bool {
        self.to_short_string() == other_str.to_string()
    }

    pub fn to_short_string(&self) -> String {
        networking::ip_port_string(self.ip, self.port)
    }

    pub fn multiple_from_string(data: String, trim: bool) -> HashSet<Node> {
        let mut nodes: HashSet<Node> = HashSet::new();
        let split_by_line = data.split("\n");
        // Skip header!
        if trim {
            for line in split_by_line.skip(1) {
                Node::insert_single_from_string(line, &mut nodes);
            }
        } else {
            for line in split_by_line {
                Node::insert_single_from_string(line, &mut nodes);
            }
        }
        nodes
    }

    fn insert_single_from_string(line: &str, nodes: &mut HashSet<Node>) {
        let node_strs: Vec<&str> = line.split(" ").collect();
        // #communications with this new node is zero!
        let node = Node::new(
            node_strs[0],
            node_strs[1],
            node_strs[2].parse::<u16>().unwrap(),
        );
        nodes.insert(node);
    }

    pub fn new_sneaky(line: &str) -> Node {
        let node_strs: Vec<&str> = line.split(":").collect();
        let mut name = String::from("Sneaky-");
        let rand_string: String = thread_rng().sample_iter(&Alphanumeric).take(8).collect();
        name.push_str(&rand_string);
        Node::new(&name, node_strs[0], node_strs[1].parse::<u16>().unwrap())
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

pub fn read_starting_nodes(file_dir: &str) -> HashSet<Node> {
    let data = fs::read_to_string(file_dir).expect("Something's wrong with the file.");
    return Node::multiple_from_string(data, false);
}
