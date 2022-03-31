
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(name = "nym-wallet-address")]
pub(crate) struct Args {
    #[clap(long)]
    pub(crate) address: String,
}

fn main() {
    let args = Args::parse();
    let b = bech32::decode(&*args.address).unwrap();
    let new_gravity_address = bech32::encode("gravity", b.1, bech32::Variant::Bech32).unwrap();
    println!("Your gravity bridge address is: {}", new_gravity_address);
}
