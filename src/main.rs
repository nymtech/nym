mod provider;

fn main() {
    provider::listening_loop().unwrap();
}
