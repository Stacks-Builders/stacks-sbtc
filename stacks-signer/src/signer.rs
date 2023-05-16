use frost_signer::config::Config;
use frost_signer::signer::{Error as SignerError, Signer as FrostSigner};

#[derive(Clone)]
pub struct Signer {
    frost_signer: FrostSigner,
    //TODO: Are there any StacksSigner specific items or maybe a stacks signer specific config that needs to be wrapped around Config?
}

impl Signer {
    pub fn new(config: Config, id: u32) -> Self {
        Self {
            frost_signer: FrostSigner::new(config, id),
        }
    }

    pub fn start_p2p_sync(&mut self) -> Result<(), SignerError> {
        self.frost_signer.start_p2p_sync()
    }
}
