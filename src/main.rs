use crate::node::MixNode;

mod mix_peer;
mod node;

fn main() {
    let mix = MixNode::new("127.0.0.1:8080", Default::default());
    mix.start_listening().unwrap();
}
