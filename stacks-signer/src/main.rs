use clap::Parser;
use frost_signer::config::Config;
use frost_signer::logging;
use stacks_signer::cli::{Cli, Command};
use stacks_signer::signer::Signer;
use tracing::info;
use wsts::Point;

fn main() {
    let cli = Cli::parse();

    // Initialize logging
    logging::initiate_tracing_subscriber().unwrap();

    // Determine what action the caller wishes to perform
    match cli.command {
        Command::Run { id, config } => {
            //TODO: getConf from sBTC contract instead
            match Config::from_path(&config) {
                Ok(config) => {
                    let mut signer = Signer::new(config, id);
                    info!("{} signer id #{}", stacks_signer::version(), id); // sign-on message
                    if let Err(e) = signer.start_p2p_sync() {
                        panic!("An error occurred on the P2P Network: {}", e);
                    }
                }
                Err(e) => {
                    panic!("An error occurred reading config file {}: {}", config, e);
                }
            }
        }
        Command::PrivateKey(secp256k1) => {
            if let Err(e) = secp256k1.generate_private_key() {
                panic!("An error occurred generating private key: {}", e);
            }
        }
        Command::PublicKey { config } => match Config::from_path(&config) {
            Ok(config) => {
                let public_key = Point::from(&config.network_private_key);
                println!("{public_key}")
            }
            Err(e) => {
                panic!("An error occurred reading config file {}: {}", config, e);
            }
        },
    };
}
