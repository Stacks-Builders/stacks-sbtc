use crate::config::{Config, SignerKeys};
use crate::net::{Error as HttpNetError, HttpNet, HttpNetListen, Message, Net, NetListen};
use crate::signing_round::{Error as SigningRoundError, MessageTypes, Signable, SigningRound};
use p256k1::ecdsa;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::spawn;
use std::{thread, time};
use tracing::{debug, warn};

// on-disk format for frost save data
#[derive(Clone)]
pub struct Signer {
    pub config: Config,
    pub signer_id: u32,
}

impl Signer {
    pub fn new(config: Config, signer_id: u32) -> Self {
        Self { config, signer_id }
    }

    pub fn start_p2p_sync(&mut self) -> Result<(), Error> {
        let signer_keys = self.config.signer_keys.clone();
        let coordinator_public_key = self.config.coordinator_public_key;

        //Create http relay
        let net: HttpNet = HttpNet::new(self.config.http_relay_url.clone());
        let net_queue = HttpNetListen::new(net.clone(), vec![]);
        // thread coordination
        let (tx, rx): (Sender<Message>, Receiver<Message>) = mpsc::channel();

        // start p2p sync
        let id = self.signer_id;
        spawn(move || poll_loop(net_queue, tx, id, signer_keys, coordinator_public_key));

        // listen to p2p messages
        self.start_signing_round(&net, rx)
    }

    fn start_signing_round(&self, net: &HttpNet, rx: Receiver<Message>) -> Result<(), Error> {
        let network_private_key = self.config.network_private_key;
        let mut round = SigningRound::from(self);
        loop {
            // Retreive a message from coordinator
            let inbound = rx.recv()?; // blocking
            let outbounds = round.process(inbound.msg)?;
            for out in outbounds {
                let msg = Message {
                    msg: out.clone(),
                    sig: match out {
                        MessageTypes::DkgBegin(msg) | MessageTypes::DkgPrivateBegin(msg) => {
                            msg.sign(&network_private_key).expect("").to_vec()
                        }
                        MessageTypes::DkgEnd(msg) | MessageTypes::DkgPublicEnd(msg) => {
                            msg.sign(&network_private_key).expect("").to_vec()
                        }
                        MessageTypes::DkgPublicShare(msg) => {
                            msg.sign(&network_private_key).expect("").to_vec()
                        }
                        MessageTypes::DkgPrivateShares(msg) => {
                            msg.sign(&network_private_key).expect("").to_vec()
                        }
                        MessageTypes::NonceRequest(msg) => {
                            msg.sign(&network_private_key).expect("").to_vec()
                        }
                        MessageTypes::NonceResponse(msg) => {
                            msg.sign(&network_private_key).expect("").to_vec()
                        }
                        MessageTypes::SignShareRequest(msg) => {
                            msg.sign(&network_private_key).expect("").to_vec()
                        }
                        MessageTypes::SignShareResponse(msg) => {
                            msg.sign(&network_private_key).expect("").to_vec()
                        }
                    },
                };
                net.send_message(msg)?;
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Http Network Error: {0}")]
    HttpNetError(#[from] HttpNetError),

    #[error("Signing Round Error: {0}")]
    SigningRoundError(#[from] SigningRoundError),

    #[error("Failed to retrieve message: {0}")]
    RecvError(#[from] mpsc::RecvError),

    #[error("Failed to send message")]
    SendError,
}

impl From<mpsc::SendError<Message>> for Error {
    fn from(_: mpsc::SendError<Message>) -> Error {
        Error::SendError
    }
}

fn poll_loop(
    mut net: HttpNetListen,
    tx: Sender<Message>,
    id: u32,
    signer_keys: SignerKeys,
    coordinator_public_key: ecdsa::PublicKey,
) -> Result<(), Error> {
    const BASE_TIMEOUT: u64 = 2;
    const MAX_TIMEOUT: u64 = 128;
    let mut timeout = BASE_TIMEOUT;
    loop {
        net.poll(id);
        match net.next_message() {
            None => {
                timeout = if timeout == 0 {
                    BASE_TIMEOUT
                } else if timeout >= MAX_TIMEOUT {
                    MAX_TIMEOUT
                } else {
                    timeout * 2
                };
            }
            Some(m) => {
                timeout = 0;
                if verify_msg(&m, &signer_keys, &coordinator_public_key) {
                    // Only send verified messages down the pipe
                    tx.send(m)?;
                }
            }
        };
        thread::sleep(time::Duration::from_millis(timeout));
    }
}

fn verify_msg(
    m: &Message,
    signer_keys: &SignerKeys,
    coordinator_public_key: &ecdsa::PublicKey,
) -> bool {
    match &m.msg {
        MessageTypes::DkgBegin(msg) | MessageTypes::DkgPrivateBegin(msg) => {
            if !msg.verify(&m.sig, coordinator_public_key) {
                warn!("Received a DkgPrivateBegin message with an invalid signature.");
                return false;
            }
        }
        MessageTypes::DkgEnd(msg) | MessageTypes::DkgPublicEnd(msg) => {
            if let Some(public_key) = signer_keys.signers.get(&msg.signer_id) {
                debug!("HERE WE GO");
                debug!("{:?}", public_key.to_bytes());

                if !msg.verify(&m.sig, public_key) {
                    warn!("Received a DkgPublicEnd message with an invalid signature.");
                    return false;
                }
            } else {
                warn!(
                    "Received a DkgPublicEnd message with an unknown id: {}",
                    msg.signer_id
                );
                return false;
            }
        }
        MessageTypes::DkgPublicShare(msg) => {
            if let Some(public_key) = signer_keys.key_ids.get(&msg.party_id) {
                if !msg.verify(&m.sig, public_key) {
                    warn!("Received a DkgPublicShare message with an invalid signature.");
                    return false;
                }
            } else {
                warn!(
                    "Received a DkgPublicShare message with an unknown id: {}",
                    msg.party_id
                );
                return false;
            }
        }
        MessageTypes::DkgPrivateShares(msg) => {
            // Private shares have key IDs from [0, N) to reference IDs from [1, N]
            // in Frost V4 to enable easy indexing hence ID + 1
            // TODO: Once Frost V5 is released, this off by one adjustment will no longer be required
            let key_id = msg.key_id + 1;
            if let Some(public_key) = signer_keys.key_ids.get(&key_id) {
                if !msg.verify(&m.sig, public_key) {
                    warn!("Received a DkgPrivateShares message with an invalid signature.");
                    return false;
                }
            } else {
                warn!(
                    "Received a DkgPrivateShares message with an unknown id: {}",
                    key_id
                );
                return false;
            }
        }
        MessageTypes::NonceRequest(msg) => {
            if !msg.verify(&m.sig, coordinator_public_key) {
                warn!("Received a NonceRequest message with an invalid signature.");
                return false;
            }
        }
        MessageTypes::NonceResponse(msg) => {
            if let Some(public_key) = signer_keys.signers.get(&msg.signer_id) {
                if !msg.verify(&m.sig, public_key) {
                    warn!("Received a NonceResponse message with an invalid signature.");
                    return false;
                }
            } else {
                warn!(
                    "Received a NonceResponse message with an unknown id: {}",
                    msg.signer_id
                );
                return false;
            }
        }
        MessageTypes::SignShareRequest(msg) => {
            if !msg.verify(&m.sig, coordinator_public_key) {
                warn!("Received a SignShareRequest message with an invalid signature.");
                return false;
            }
        }
        MessageTypes::SignShareResponse(msg) => {
            if let Some(public_key) = signer_keys.signers.get(&msg.signer_id) {
                if !msg.verify(&m.sig, public_key) {
                    warn!("Received a SignShareResponse message with an invalid signature.");
                    return false;
                }
            } else {
                warn!(
                    "Received a SignShareResponse message with an unknown id: {}",
                    msg.signer_id
                );
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod test {
    use hashbrown::HashMap;
    use p256k1::ecdsa::PublicKey;
    use rand_core::OsRng;
    use wsts::{
        common::{PolyCommitment, PublicNonce},
        schnorr::ID,
        Scalar,
    };

    use crate::{
        config::SignerKeys,
        net::Message,
        signing_round::{
            DkgBegin, DkgEnd, DkgPrivateShares, DkgPublicShare, DkgStatus, MessageTypes,
            NonceRequest, NonceResponse, Signable, SignatureShareRequest, SignatureShareResponse,
        },
    };

    use super::verify_msg;

    fn generate_key_pair() -> (Scalar, PublicKey) {
        // Generate a secret and public key
        let mut rnd = OsRng::default();
        let sec_key = Scalar::random(&mut rnd);
        let pub_key = PublicKey::new(&sec_key).unwrap();
        (sec_key, pub_key)
    }

    struct TestConfig {
        coordinator_sec_key: Scalar,
        coordinator_pub_key: PublicKey,
        sec_keys: Vec<Scalar>,
        signer_keys: SignerKeys,
    }

    impl TestConfig {
        pub fn new() -> Self {
            let (coordinator_sec_key, coordinator_pub_key) = generate_key_pair();

            let (sec_key1, pub_key1) = generate_key_pair();
            let (sec_key2, pub_key2) = generate_key_pair();

            let signer_keys = SignerKeys {
                signers: HashMap::from([(1, pub_key1), (2, pub_key2)]),
                key_ids: HashMap::from([
                    (1, pub_key1),
                    (2, pub_key1),
                    (3, pub_key2),
                    (4, pub_key2),
                ]),
            };
            Self {
                coordinator_sec_key,
                coordinator_pub_key,
                sec_keys: [sec_key1, sec_key2].to_vec(),
                signer_keys,
            }
        }
    }

    #[test]
    fn verify_msg_dkg_begin() {
        let config = TestConfig::new();
        //Test DkgBegin && DkgPrivateBegin branch
        let inner = DkgBegin { dkg_id: 0 };
        let sig = inner.sign(&config.coordinator_sec_key).unwrap();
        // DkgBegin
        let msg = MessageTypes::DkgBegin(inner.clone());
        let dkg_begin = Message {
            msg,
            sig: sig.clone(),
        };
        // DkgPrivateBegin
        let msg = MessageTypes::DkgPrivateBegin(inner);
        let dkg_private_begin = Message { msg, sig };

        // Check with correct public key
        assert!(verify_msg(
            &dkg_begin,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
        assert!(verify_msg(
            &dkg_private_begin,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));

        // Check with incorrect public key
        assert!(!verify_msg(
            &dkg_begin,
            &config.signer_keys,
            &config.signer_keys.key_ids.get(&1).unwrap(),
        ));
        assert!(!verify_msg(
            &dkg_private_begin,
            &config.signer_keys,
            &config.signer_keys.key_ids.get(&1).unwrap(),
        ));
    }

    #[test]
    fn verify_msg_dkg_end_valid_signer_id() {
        let config = TestConfig::new();
        //Test DkgEnd and DkgPublicEnd
        let inner = DkgEnd {
            dkg_id: 0,
            signer_id: 1,
            status: DkgStatus::Success,
        };
        let sig = inner.sign(&config.sec_keys[0]).unwrap();
        let msg = MessageTypes::DkgEnd(inner.clone());
        let dkg_end = Message {
            msg,
            sig: sig.clone(),
        };
        let msg = MessageTypes::DkgPublicEnd(inner.clone());
        let dkg_public_end = Message {
            msg: msg.clone(),
            sig,
        };

        assert!(verify_msg(
            &dkg_end,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));

        assert!(verify_msg(
            &dkg_public_end,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));

        //Let us sign with the wrong sec key...
        let sig = inner.sign(&config.sec_keys[1]).unwrap();
        let dkg_end = Message {
            msg,
            sig: sig.clone(),
        };
        let msg = MessageTypes::DkgPublicEnd(inner);
        let dkg_public_end = Message { msg, sig };

        assert!(!verify_msg(
            &dkg_end,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));

        assert!(!verify_msg(
            &dkg_public_end,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
    }

    #[test]
    fn verify_msg_dkg_end_invalid_signer_id() {
        let config = TestConfig::new();
        //Test DkgEnd and DkgPublicEnd
        let invalid_inner = DkgEnd {
            dkg_id: 0,
            signer_id: 3, // We have a signer_id that does not match any known signers...
            status: DkgStatus::Success,
        };
        let sig = invalid_inner.sign(&config.sec_keys[0]).unwrap();

        let msg = MessageTypes::DkgEnd(invalid_inner.clone());
        let dkg_end = Message {
            msg,
            sig: sig.clone(),
        };
        let msg = MessageTypes::DkgPublicEnd(invalid_inner.clone());
        let dkg_public_end = Message { msg, sig: sig };

        assert!(!verify_msg(
            &dkg_end,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
        assert!(!verify_msg(
            &dkg_public_end,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
    }

    #[test]
    fn verify_msg_dkg_public_share_valid_party_id() {
        let config = TestConfig::new();
        let mut rng = OsRng::default();
        let inner = DkgPublicShare {
            dkg_id: 0,
            dkg_public_id: 0,
            party_id: 1,
            public_share: PolyCommitment {
                id: ID::new(&Scalar::new(), &Scalar::new(), &mut rng),
                A: vec![],
            },
        };

        let msg = MessageTypes::DkgPublicShare(inner.clone());
        let sig = inner.sign(&config.sec_keys[0]).unwrap();

        let message = Message {
            msg: msg.clone(),
            sig,
        };
        assert!(verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));

        // Let's sign with the wrong sec key...
        let sig = inner.sign(&config.sec_keys[1]).unwrap();
        let msg = MessageTypes::DkgPublicShare(inner);

        let message = Message { msg, sig };
        assert!(!verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
    }

    #[test]
    fn verify_msg_dkg_public_share_invalid_party_id() {
        let config = TestConfig::new();
        let mut rng = OsRng::default();
        let inner = DkgPublicShare {
            dkg_id: 0,
            dkg_public_id: 0,
            party_id: 10, // We don't know this part id..
            public_share: PolyCommitment {
                id: ID::new(&Scalar::new(), &Scalar::new(), &mut rng),
                A: vec![],
            },
        };

        let msg = MessageTypes::DkgPublicShare(inner.clone());
        let sig = inner.sign(&config.sec_keys[0]).unwrap();

        let message = Message { msg, sig };
        assert!(!verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
    }

    #[test]
    fn verify_msg_dkg_private_share_valid_key_id() {
        let config = TestConfig::new();
        let inner = DkgPrivateShares {
            dkg_id: 0,
            key_id: 1,
            private_shares: HashMap::new(),
        };
        let sig = inner.sign(&config.sec_keys[0]).unwrap();
        let msg = MessageTypes::DkgPrivateShares(inner.clone());

        let message = Message {
            msg: msg.clone(),
            sig,
        };
        assert!(verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));

        // Let us sign with the wrong sec key...
        let sig = inner.sign(&config.sec_keys[1]).unwrap();
        let message = Message { msg, sig };

        assert!(!verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
    }

    #[test]
    fn verify_msg_dkg_private_share_invalid_key_id() {
        let config = TestConfig::new();
        let inner = DkgPrivateShares {
            dkg_id: 0,
            key_id: 10, // we don't know this key id...
            private_shares: HashMap::new(),
        };
        let sig = inner.sign(&config.sec_keys[0]).unwrap();
        let msg = MessageTypes::DkgPrivateShares(inner);

        let message = Message { msg, sig };
        assert!(!verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
    }

    #[test]
    fn verify_msg_nonce_request() {
        let config = TestConfig::new();

        let inner = NonceRequest {
            dkg_id: 0,
            sign_id: 0,
            sign_nonce_id: 0,
        };

        let sig = inner.sign(&config.coordinator_sec_key).unwrap();
        let msg = MessageTypes::NonceRequest(inner);

        let message = Message { msg, sig };
        assert!(verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
        // Let's check with the wrong pub key
        assert!(!verify_msg(
            &message,
            &config.signer_keys,
            &config.signer_keys.key_ids.get(&1).unwrap(),
        ));
    }

    #[test]
    fn verify_msg_nonce_response_valid_signer_id() {
        let config = TestConfig::new();
        let inner = NonceResponse {
            dkg_id: 0,
            sign_id: 0,
            sign_nonce_id: 0,
            signer_id: 1,
            key_ids: vec![],
            nonces: vec![],
        };
        let sig = inner.sign(&config.sec_keys[0]).unwrap();
        let msg = MessageTypes::NonceResponse(inner.clone());

        let message = Message {
            msg: msg.clone(),
            sig,
        };
        assert!(verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));

        // Let's sign with the wrong sec key...
        let sig = inner.sign(&config.sec_keys[1]).unwrap();

        let message = Message { msg, sig };
        assert!(!verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
    }

    #[test]
    fn verify_msg_nonce_response_invalid_signer_id() {
        let config = TestConfig::new();
        let inner = NonceResponse {
            dkg_id: 0,
            sign_id: 0,
            sign_nonce_id: 0,
            signer_id: 10, // We don't have 10 signers...
            key_ids: vec![],
            nonces: vec![],
        };
        let sig = inner.sign(&config.sec_keys[0]).unwrap();
        let msg = MessageTypes::NonceResponse(inner);

        let message = Message { msg, sig };
        assert!(!verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
    }

    #[test]
    fn verify_msg_sign_share_request() {
        let config = TestConfig::new();
        let inner = SignatureShareRequest {
            dkg_id: 0,
            sign_id: 0,
            correlation_id: 0,
            nonce_responses: vec![NonceResponse {
                dkg_id: 0,
                sign_id: 0,
                sign_nonce_id: 0,
                signer_id: 1,
                key_ids: vec![0],
                nonces: vec![PublicNonce {
                    D: Default::default(),
                    E: Default::default(),
                }],
            }],
            message: vec![],
        };
        let sig = inner.sign(&config.coordinator_sec_key).unwrap();
        let msg = MessageTypes::SignShareRequest(inner);

        let message = Message { msg, sig };
        assert!(verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));

        // Let's check the wrong pub key...
        assert!(!verify_msg(
            &message,
            &config.signer_keys,
            &config.signer_keys.key_ids.get(&1).unwrap()
        ));
    }

    #[test]
    fn verify_msg_sign_share_response_valid_signer_id() {
        // SignShareResponse(SignatureShareResponse),
        let config = TestConfig::new();
        let inner = SignatureShareResponse {
            dkg_id: 0,
            sign_id: 0,
            correlation_id: 0,
            signer_id: 1,
            signature_shares: vec![],
        };
        let sig = inner.sign(&config.sec_keys[0]).unwrap();
        let msg = MessageTypes::SignShareResponse(inner.clone());

        let message = Message {
            msg: msg.clone(),
            sig,
        };
        assert!(verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));

        // Let's sign with the wrong sec key...
        let sig = inner.sign(&config.sec_keys[1]).unwrap();
        let message = Message { msg, sig };
        assert!(!verify_msg(
            &message,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
    }

    #[test]
    fn verify_msg_sign_share_response_invalid_signer_id() {
        let config = TestConfig::new();
        let inner = SignatureShareResponse {
            dkg_id: 0,
            sign_id: 0,
            correlation_id: 0,
            signer_id: 10,
            signature_shares: vec![],
        };
        let sig = inner.sign(&config.sec_keys[0]).unwrap();
        let msg = MessageTypes::SignShareResponse(inner);

        let sign_share_response = Message { msg, sig };
        assert!(!verify_msg(
            &sign_share_response,
            &config.signer_keys,
            &config.coordinator_pub_key
        ));
    }
}
