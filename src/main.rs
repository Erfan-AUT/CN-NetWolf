mod node;
mod udp;

fn main() -> std::io::Result<()> {
    let _nodes = node::read_starting_nodes();
    let a = _nodes[0].to_string();
    println!("{}", a);
    Ok(())
}

