use clap::Parser;
use relay_server::Server;

fn main() {
    let args = Cli::parse();
    Server::run(&args.listen);
}

#[derive(Parser)]
struct Cli {
    /// Where to listen for incoming connections
    #[clap(short, long, default_value = "127.0.0.1:9776")]
    listen: String,
}
