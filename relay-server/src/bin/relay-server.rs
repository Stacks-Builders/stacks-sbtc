use relay_server::Server;

fn main() {
    let addr = "127.0.0.1:9776";
    Server::run(addr);
}
