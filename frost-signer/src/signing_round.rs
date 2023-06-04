use hashbrown::{HashMap, HashSet};
use p256k1::{
    ecdsa,
    point::{Compressed, Point},
    scalar::Scalar,
};
use rand_core::{CryptoRng, OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use tracing::{debug, info, warn};
pub use wsts;
use wsts::{
    common::{PolyCommitment, PublicNonce, SignatureShare},
    traits::Signer as SignerTrait,
    v1,
};

use crate::{
    config::PublicKeys,
    signer::Signer as FrostSigner,
    state_machine::{Error as StateMachineError, StateMachine, States},
    util::{decrypt, encrypt, make_shared_secret},
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("InvalidPartyID")]
    InvalidPartyID,
    #[error("InvalidDkgPublicShare")]
    InvalidDkgPublicShare,
    #[error("InvalidDkgPrivateShares")]
    InvalidDkgPrivateShares(Vec<u32>),
    #[error("InvalidNonceResponse")]
    InvalidNonceResponse,
    #[error("InvalidSignatureShare")]
    InvalidSignatureShare,
    #[error("State Machine Error: {0}")]
    StateMachineError(#[from] StateMachineError),
}

pub trait Signable {
    fn hash(&self, hasher: &mut Sha256);

    fn sign(&self, private_key: &Scalar) -> Result<Vec<u8>, ecdsa::Error> {
        let mut hasher = Sha256::new();

        self.hash(&mut hasher);

        let hash = hasher.finalize();
        match ecdsa::Signature::new(hash.as_slice(), private_key) {
            Ok(sig) => Ok(sig.to_bytes().to_vec()),
            Err(e) => Err(e),
        }
    }

    fn verify(&self, signature: &[u8], public_key: &ecdsa::PublicKey) -> bool {
        let mut hasher = Sha256::new();

        self.hash(&mut hasher);

        let hash = hasher.finalize();
        let sig = match ecdsa::Signature::try_from(signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        sig.verify(hash.as_slice(), public_key)
    }
}

pub struct SigningRound {
    pub dkg_id: u64,
    pub dkg_public_id: u64,
    pub sign_id: u64,
    pub sign_nonce_id: u64,
    pub threshold: u32,
    pub total_signers: u32,
    pub total_keys: u32,
    pub signer: Signer,
    pub state: States,
    pub commitments: BTreeMap<u32, PolyCommitment>,
    pub shares: HashMap<u32, HashMap<u32, Vec<u8>>>,
    pub public_nonces: Vec<PublicNonce>,
    pub network_private_key: Scalar,
    pub public_keys: PublicKeys,
}

pub struct Signer {
    pub frost_signer: wsts::v1::Signer,
    pub signer_id: u32,
}

impl StateMachine for SigningRound {
    fn move_to(&mut self, state: States) -> Result<(), StateMachineError> {
        self.can_move_to(&state)?;
        self.state = state;
        Ok(())
    }

    fn can_move_to(&self, state: &States) -> Result<(), StateMachineError> {
        let prev_state = &self.state;
        let accepted = match state {
            States::Idle => true,
            States::DkgPublicDistribute => {
                prev_state == &States::Idle
                    || prev_state == &States::DkgPublicGather
                    || prev_state == &States::DkgPrivateDistribute
            }
            States::DkgPublicGather => prev_state == &States::DkgPublicDistribute,
            States::DkgPrivateDistribute => prev_state == &States::DkgPublicGather,
            States::DkgPrivateGather => prev_state == &States::DkgPrivateDistribute,
            States::SignGather => prev_state == &States::Idle,
            States::Signed => prev_state == &States::SignGather,
        };
        if accepted {
            info!("state change from {:?} to {:?}", prev_state, state);
            Ok(())
        } else {
            Err(StateMachineError::BadStateChange(format!(
                "{:?} to {:?}",
                prev_state, state
            )))
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DkgStatus {
    Success,
    Failure(String),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum MessageTypes {
    DkgBegin(DkgBegin),
    DkgPrivateBegin(DkgBegin),
    DkgEnd(DkgEnd),
    DkgPublicEnd(DkgEnd),
    DkgPublicShare(DkgPublicShare),
    DkgPrivateShares(DkgPrivateShares),
    NonceRequest(NonceRequest),
    NonceResponse(NonceResponse),
    SignShareRequest(SignatureShareRequest),
    SignShareResponse(SignatureShareResponse),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DkgPublicShare {
    pub dkg_id: u64,
    pub dkg_public_id: u64,
    pub party_id: u32,
    pub public_share: PolyCommitment,
}

impl Signable for DkgPublicShare {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DKG_PUBLIC_SHARE".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.dkg_public_id.to_be_bytes());
        hasher.update(self.party_id.to_be_bytes());
        for a in &self.public_share.A {
            hasher.update(a.compress().as_bytes());
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DkgPrivateShares {
    pub dkg_id: u64,
    pub key_id: u32,
    /// Encrypt the shares using AES-GCM with a key derived from ECDH
    pub private_shares: HashMap<u32, Vec<u8>>,
}

impl Signable for DkgPrivateShares {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DKG_PRIVATE_SHARES".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.key_id.to_be_bytes());
        // make sure we iterate sequentially
        // TODO: change this once WSTS goes to 1 based indexing for key_ids, or change to BTreeMap
        for id in 0..self.private_shares.len() as u32 {
            hasher.update(id.to_be_bytes());
            hasher.update(&self.private_shares[&id]);
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DkgBegin {
    pub dkg_id: u64, //TODO: Strong typing for this, alternatively introduce a type alias
}

impl Signable for DkgBegin {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DKG_BEGIN".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DkgEnd {
    pub dkg_id: u64,
    pub signer_id: u32,
    pub status: DkgStatus,
}

impl Signable for DkgEnd {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DKG_END".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.signer_id.to_be_bytes());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NonceRequest {
    pub dkg_id: u64,
    pub sign_id: u64,
    pub sign_nonce_id: u64,
}

impl Signable for NonceRequest {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("NONCE_REQUEST".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.sign_id.to_be_bytes());
        hasher.update(self.sign_nonce_id.to_be_bytes());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NonceResponse {
    pub dkg_id: u64,
    pub sign_id: u64,
    pub sign_nonce_id: u64,
    pub signer_id: u32,
    pub key_ids: Vec<u32>,
    pub nonces: Vec<PublicNonce>,
}

impl Signable for NonceResponse {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("NONCE_RESPONSE".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.sign_id.to_be_bytes());
        hasher.update(self.sign_nonce_id.to_be_bytes());
        hasher.update(self.signer_id.to_be_bytes());

        for key_id in &self.key_ids {
            hasher.update(key_id.to_be_bytes());
        }

        for nonce in &self.nonces {
            hasher.update(nonce.D.compress().as_bytes());
            hasher.update(nonce.E.compress().as_bytes());
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SignatureShareRequest {
    pub dkg_id: u64,
    pub sign_id: u64,
    pub correlation_id: u64,
    pub nonce_responses: Vec<NonceResponse>,
    pub message: Vec<u8>,
}

impl Signable for SignatureShareRequest {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("SIGNATURE_SHARE_REQUEST".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.sign_id.to_be_bytes());
        hasher.update(self.correlation_id.to_be_bytes());

        for nonce_response in &self.nonce_responses {
            nonce_response.hash(hasher);
        }

        hasher.update(self.message.as_slice());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SignatureShareResponse {
    pub dkg_id: u64,
    pub sign_id: u64,
    pub correlation_id: u64,
    pub signer_id: u32,
    pub signature_shares: Vec<SignatureShare>,
}

impl Signable for SignatureShareResponse {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("SIGNATURE_SHARE_RESPONSE".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.sign_id.to_be_bytes());
        hasher.update(self.correlation_id.to_be_bytes());
        hasher.update(self.signer_id.to_be_bytes());

        for signature_share in &self.signature_shares {
            hasher.update(signature_share.id.to_be_bytes());
            hasher.update(signature_share.z_i.to_bytes());
        }
    }
}

impl SigningRound {
    pub fn new(
        threshold: u32,
        total_signers: u32,
        total_keys: u32,
        signer_id: u32,
        key_ids: Vec<u32>,
        network_private_key: Scalar,
        public_keys: PublicKeys,
    ) -> SigningRound {
        assert!(threshold <= total_keys);
        let mut rng = OsRng::default();
        let frost_signer = v1::Signer::new(signer_id, &key_ids, total_keys, threshold, &mut rng);
        let signer = Signer {
            frost_signer,
            signer_id,
        };

        SigningRound {
            dkg_id: 0,
            dkg_public_id: 0,
            sign_id: 1,
            sign_nonce_id: 1,
            threshold,
            total_signers,
            total_keys,
            signer,
            state: States::Idle,
            commitments: BTreeMap::new(),
            shares: HashMap::new(),
            public_nonces: vec![],
            network_private_key,
            public_keys,
        }
    }

    fn reset<T: RngCore + CryptoRng>(&mut self, dkg_id: u64, rng: &mut T) {
        self.dkg_id = dkg_id;
        self.dkg_public_id = 0;
        self.commitments.clear();
        self.shares.clear();
        self.public_nonces.clear();
        self.signer.frost_signer.reset_polys(rng);
    }

    pub fn process(&mut self, message: MessageTypes) -> Result<Vec<MessageTypes>, Error> {
        let out_msgs = match message {
            MessageTypes::DkgBegin(dkg_begin) => self.dkg_begin(dkg_begin),
            MessageTypes::DkgPrivateBegin(_) => self.dkg_private_begin(),
            MessageTypes::DkgPublicShare(dkg_public_shares) => {
                self.dkg_public_share(dkg_public_shares)
            }
            MessageTypes::DkgPrivateShares(dkg_private_shares) => {
                self.dkg_private_shares(dkg_private_shares)
            }
            MessageTypes::SignShareRequest(sign_share_request) => {
                self.sign_share_request(sign_share_request)
            }
            MessageTypes::NonceRequest(nonce_request) => self.nonce_request(nonce_request),
            _ => Ok(vec![]), // TODO
        };

        match out_msgs {
            Ok(mut out) => {
                if self.public_shares_done() {
                    debug!(
                        "public_shares_done==true. commitments {}",
                        self.commitments.len()
                    );
                    let dkg_end_msgs = self.dkg_public_ended()?;
                    out.push(dkg_end_msgs);
                    self.move_to(States::DkgPrivateDistribute)?;
                } else if self.can_dkg_end() {
                    debug!(
                        "can_dkg_end==true. shares {} commitments {}",
                        self.shares.len(),
                        self.commitments.len()
                    );
                    let dkg_end_msgs = self.dkg_ended()?;
                    out.push(dkg_end_msgs);
                    self.move_to(States::Idle)?;
                }
                Ok(out)
            }
            Err(e) => Err(e),
        }
    }

    fn dkg_public_ended(&mut self) -> Result<MessageTypes, Error> {
        let dkg_end = DkgEnd {
            dkg_id: self.dkg_id,
            signer_id: self.signer.signer_id,
            status: DkgStatus::Success,
        };
        let dkg_end = MessageTypes::DkgPublicEnd(dkg_end);
        info!(
            "DKG_END round #{} signer_id {}",
            self.dkg_id, self.signer.signer_id
        );
        Ok(dkg_end)
    }

    fn dkg_ended(&mut self) -> Result<MessageTypes, Error> {
        let polys: Vec<PolyCommitment> = self.commitments.clone().into_values().collect();

        let mut decrypted_shares = HashMap::new();

        // go through private shares, and decrypt any for owned keys, leaving the rest as zero scalars
        let key_ids: HashSet<u32> = self.signer.frost_signer.get_key_ids().into_iter().collect();
        let mut invalid_dkg_private_shares = Vec::new();

        for (src_key_id, encrypted_shares) in &self.shares {
            let mut decrypted_key_shares = HashMap::new();

            for (dst_key_id, private_share) in encrypted_shares {
                if key_ids.contains(dst_key_id) {
                    debug!(
                        "decrypting dkg private share for key_id #{}",
                        dst_key_id + 1
                    );
                    let compressed =
                        Compressed::from(self.public_keys.key_ids[&(src_key_id + 1)].to_bytes());
                    let src_public_key = Point::try_from(&compressed).unwrap();
                    let shared_secret =
                        make_shared_secret(&self.network_private_key, &src_public_key);

                    match decrypt(&shared_secret, private_share) {
                        Ok(plain) => match Scalar::try_from(&plain[..]) {
                            Ok(s) => {
                                decrypted_key_shares.insert(*dst_key_id, s);
                            }
                            Err(e) => {
                                warn!("Failed to parse Scalar for dkg private share from key_id {} to key_id {}: {:?}", src_key_id, dst_key_id, e);
                                invalid_dkg_private_shares.push(*src_key_id);
                            }
                        },
                        Err(e) => {
                            warn!("Failed to decrypt dkg private share from key_id {} to key_id {}: {:?}", src_key_id, dst_key_id, e);
                            invalid_dkg_private_shares.push(*src_key_id);
                        }
                    }
                } else {
                    decrypted_key_shares.insert(*dst_key_id, Scalar::new());
                }
            }

            decrypted_shares.insert(*src_key_id, decrypted_key_shares);
        }

        let dkg_end = if invalid_dkg_private_shares.is_empty() {
            match self
                .signer
                .frost_signer
                .compute_secrets(&decrypted_shares, &polys)
            {
                Ok(()) => DkgEnd {
                    dkg_id: self.dkg_id,
                    signer_id: self.signer.signer_id,
                    status: DkgStatus::Success,
                },
                Err(dkg_error_map) => DkgEnd {
                    dkg_id: self.dkg_id,
                    signer_id: self.signer.signer_id,
                    status: DkgStatus::Failure(format!("{:?}", dkg_error_map)),
                },
            }
        } else {
            DkgEnd {
                dkg_id: self.dkg_id,
                signer_id: self.signer.signer_id,
                status: DkgStatus::Failure(format!("{:?}", invalid_dkg_private_shares)),
            }
        };

        let dkg_end = MessageTypes::DkgEnd(dkg_end);
        info!(
            "DKG_END round #{} signer_id {}",
            self.dkg_id, self.signer.signer_id
        );
        Ok(dkg_end)
    }

    fn public_shares_done(&self) -> bool {
        debug!(
            "public_shares_done state {:?} commitments {}",
            self.state,
            self.commitments.len(),
        );
        self.state == States::DkgPublicGather
            && self.commitments.len() == usize::try_from(self.total_keys).unwrap()
    }

    fn can_dkg_end(&self) -> bool {
        debug!(
            "can_dkg_end state {:?} commitments {} shares {}",
            self.state,
            self.commitments.len(),
            self.shares.len()
        );
        self.state == States::DkgPrivateGather
            && self.commitments.len() == usize::try_from(self.total_keys).unwrap()
            && self.shares.len() == usize::try_from(self.total_keys).unwrap()
    }

    fn nonce_request(&mut self, nonce_request: NonceRequest) -> Result<Vec<MessageTypes>, Error> {
        let mut rng = OsRng::default();
        let mut msgs = vec![];
        let signer_id = self.signer.signer_id;
        let key_ids = self.signer.frost_signer.get_key_ids();
        let nonces = self.signer.frost_signer.gen_nonces(&mut rng);

        let response = NonceResponse {
            dkg_id: nonce_request.dkg_id,
            sign_id: nonce_request.sign_id,
            sign_nonce_id: nonce_request.sign_nonce_id,
            signer_id,
            key_ids,
            nonces,
        };

        let response = MessageTypes::NonceResponse(response);

        info!(
            "nonce request with dkg_id {:?}. response sent from signer_id {}",
            nonce_request.dkg_id, signer_id
        );
        msgs.push(response);

        Ok(msgs)
    }

    fn sign_share_request(
        &mut self,
        sign_request: SignatureShareRequest,
    ) -> Result<Vec<MessageTypes>, Error> {
        let mut msgs = vec![];

        let signer_ids = sign_request
            .nonce_responses
            .iter()
            .map(|nr| nr.signer_id)
            .collect::<Vec<u32>>();

        info!("Got SignatureShareRequest for signer_ids {:?}", signer_ids);

        for signer_id in &signer_ids {
            if *signer_id == self.signer.signer_id {
                let key_ids: Vec<u32> = sign_request
                    .nonce_responses
                    .iter()
                    .flat_map(|nr| nr.key_ids.iter().copied())
                    .collect::<Vec<u32>>();
                let nonces = sign_request
                    .nonce_responses
                    .iter()
                    .flat_map(|nr| nr.nonces.clone())
                    .collect::<Vec<PublicNonce>>();
                let signature_shares = self.signer.frost_signer.sign(
                    &sign_request.message,
                    &signer_ids,
                    &key_ids,
                    &nonces,
                );

                let response = SignatureShareResponse {
                    dkg_id: sign_request.dkg_id,
                    sign_id: sign_request.sign_id,
                    correlation_id: sign_request.correlation_id,
                    signer_id: *signer_id,
                    signature_shares,
                };

                info!(
                    "Sending SignatureShareResponse for signer_id {:?}",
                    signer_id
                );

                let response = MessageTypes::SignShareResponse(response);

                msgs.push(response);
            } else {
                debug!("SignShareRequest for {} dropped.", signer_id);
            }
        }
        Ok(msgs)
    }

    fn dkg_begin(&mut self, dkg_begin: DkgBegin) -> Result<Vec<MessageTypes>, Error> {
        let mut rng = OsRng::default();

        self.reset(dkg_begin.dkg_id, &mut rng);
        self.move_to(States::DkgPublicDistribute)?;

        let _party_state = self.signer.frost_signer.save();

        self.dkg_public_begin()
    }

    fn dkg_public_begin(&mut self) -> Result<Vec<MessageTypes>, Error> {
        let mut rng = OsRng::default();
        let mut msgs = vec![];
        let polys = self.signer.frost_signer.get_poly_commitments(&mut rng);

        info!(
            "sending DkgPublicShares for round #{}, {} poly commitments for signer #{}",
            self.dkg_id,
            polys.len(),
            self.signer.frost_signer.get_id(),
        );

        for poly in &polys {
            let public_share = DkgPublicShare {
                dkg_id: self.dkg_id,
                dkg_public_id: self.dkg_public_id,
                party_id: poly.id.id.get_u32(),
                public_share: poly.clone(),
            };

            let public_share = MessageTypes::DkgPublicShare(public_share);
            msgs.push(public_share);
        }

        self.move_to(States::DkgPublicGather)?;
        Ok(msgs)
    }

    fn dkg_private_begin(&mut self) -> Result<Vec<MessageTypes>, Error> {
        let mut rng = OsRng::default();
        let mut msgs = vec![];
        for (key_id, private_shares) in &self.signer.frost_signer.get_shares() {
            info!(
                "signer {} sending dkg private share for key_id #{}",
                self.signer.signer_id, key_id
            );
            // encrypt each share for the recipient
            let mut encrypted_shares = HashMap::new();

            for (dst_key_id, private_share) in private_shares {
                debug!(
                    "encrypting dkg private share for key_id #{}",
                    dst_key_id + 1
                );
                let compressed =
                    Compressed::from(self.public_keys.key_ids[&(dst_key_id + 1)].to_bytes());
                let dst_public_key = Point::try_from(&compressed).unwrap();
                let shared_secret = make_shared_secret(&self.network_private_key, &dst_public_key);
                let encrypted_share =
                    encrypt(&shared_secret, &private_share.to_bytes(), &mut rng).unwrap();

                encrypted_shares.insert(*dst_key_id, encrypted_share);
            }

            let private_shares = DkgPrivateShares {
                dkg_id: self.dkg_id,
                key_id: *key_id,
                private_shares: encrypted_shares,
            };

            let private_shares = MessageTypes::DkgPrivateShares(private_shares);
            msgs.push(private_shares);
        }

        self.move_to(States::DkgPrivateGather)?;
        Ok(msgs)
    }

    fn dkg_public_share(
        &mut self,
        dkg_public_share: DkgPublicShare,
    ) -> Result<Vec<MessageTypes>, Error> {
        self.commitments
            .insert(dkg_public_share.party_id, dkg_public_share.public_share);
        info!(
            "received DkgPublicShare from key #{} {}/{}",
            dkg_public_share.party_id,
            self.commitments.len(),
            self.total_keys
        );
        Ok(vec![])
    }

    fn dkg_private_shares(
        &mut self,
        dkg_private_shares: DkgPrivateShares,
    ) -> Result<Vec<MessageTypes>, Error> {
        let shares_clone = dkg_private_shares.private_shares.clone();
        self.shares
            .insert(dkg_private_shares.key_id, dkg_private_shares.private_shares);
        info!(
            "received DkgPrivateShares from key #{} {}/{} {:?}",
            dkg_private_shares.key_id,
            self.shares.len(),
            self.total_keys,
            shares_clone.keys(),
        );
        Ok(vec![])
    }
}

impl From<&FrostSigner> for SigningRound {
    fn from(signer: &FrostSigner) -> Self {
        let signer_id = signer.signer_id;
        assert!(signer_id > 0 && signer_id <= signer.config.total_signers);
        let key_ids = signer.config.signer_key_ids[&signer_id]
            .iter()
            .map(|i| i - 1)
            .collect::<Vec<u32>>();

        assert!(signer.config.keys_threshold <= signer.config.total_keys);
        let mut rng = OsRng::default();
        let frost_signer = v1::Signer::new(
            signer_id,
            &key_ids,
            signer.config.total_keys,
            signer.config.keys_threshold,
            &mut rng,
        );

        let network_private_key = signer.config.network_private_key;
        let public_keys = signer.config.public_keys.clone();

        SigningRound {
            dkg_id: 1,
            dkg_public_id: 1,
            sign_id: 1,
            sign_nonce_id: 1,
            threshold: signer.config.keys_threshold,
            total_keys: signer.config.total_keys,
            total_signers: signer.config.total_signers,
            signer: Signer {
                frost_signer,
                signer_id,
            },
            state: States::Idle,
            commitments: BTreeMap::new(),
            shares: HashMap::new(),
            public_nonces: vec![],
            network_private_key,
            public_keys,
        }
    }
}

#[cfg(test)]
mod test {
    use hashbrown::HashMap;
    use rand_core::{CryptoRng, OsRng, RngCore};
    use wsts::{common::PolyCommitment, schnorr::ID, Scalar};

    use crate::signing_round::{
        DkgPrivateShares, DkgPublicShare, DkgStatus, MessageTypes, SigningRound,
    };
    use crate::state_machine::States;

    fn get_rng() -> impl RngCore + CryptoRng {
        let rnd = OsRng::default();
        //rand::rngs::StdRng::seed_from_u64(rnd.next_u64()) // todo: fix trait `rand_core::RngCore` is not implemented for `StdRng`
        rnd
    }

    #[test]
    fn dkg_public_share() {
        let mut rnd = get_rng();
        let mut signing_round =
            SigningRound::new(1, 1, 1, 1, vec![1], Default::default(), Default::default());
        let public_share = DkgPublicShare {
            dkg_id: 0,
            party_id: 0,
            public_share: PolyCommitment {
                id: ID::new(&Scalar::new(), &Scalar::new(), &mut rnd),
                A: vec![],
            },
            dkg_public_id: 0,
        };
        signing_round.dkg_public_share(public_share).unwrap();
        assert_eq!(1, signing_round.commitments.len())
    }

    #[test]
    fn dkg_private_shares() {
        let mut signing_round =
            SigningRound::new(1, 1, 1, 1, vec![1], Default::default(), Default::default());
        let mut private_shares = DkgPrivateShares {
            dkg_id: 0,
            key_id: 0,
            private_shares: HashMap::new(),
        };
        private_shares.private_shares.insert(1, Vec::new());
        signing_round.dkg_private_shares(private_shares).unwrap();
        assert_eq!(1, signing_round.shares.len())
    }

    #[test]
    fn public_shares_done() {
        let mut rnd = get_rng();
        let mut signing_round =
            SigningRound::new(1, 1, 1, 1, vec![1], Default::default(), Default::default());
        // publich_shares_done starts out as false
        assert_eq!(false, signing_round.public_shares_done());

        // meet the conditions for all public keys received
        signing_round.state = States::DkgPublicGather;
        signing_round.commitments.insert(
            1,
            PolyCommitment {
                id: ID::new(&Scalar::new(), &Scalar::new(), &mut rnd),
                A: vec![],
            },
        );

        // public_shares_done should be true
        assert!(signing_round.public_shares_done());
    }

    #[test]
    fn can_dkg_end() {
        let mut rnd = get_rng();
        let mut signing_round =
            SigningRound::new(1, 1, 1, 1, vec![1], Default::default(), Default::default());
        // can_dkg_end starts out as false
        assert_eq!(false, signing_round.can_dkg_end());

        // meet the conditions for DKG_END
        signing_round.state = States::DkgPrivateGather;
        signing_round.commitments.insert(
            1,
            PolyCommitment {
                id: ID::new(&Scalar::new(), &Scalar::new(), &mut rnd),
                A: vec![],
            },
        );
        let shares: HashMap<u32, Vec<u8>> = HashMap::new();
        signing_round.shares.insert(1, shares);

        // can_dkg_end should be true
        assert!(signing_round.can_dkg_end());
    }

    #[test]
    fn dkg_ended() {
        let mut signing_round =
            SigningRound::new(1, 1, 1, 1, vec![1], Default::default(), Default::default());
        match signing_round.dkg_ended() {
            Ok(dkg_end) => match dkg_end {
                MessageTypes::DkgEnd(dkg_end) => match dkg_end.status {
                    DkgStatus::Failure(_) => assert!(true),
                    _ => assert!(false),
                },
                _ => assert!(false),
            },
            _ => assert!(false),
        }
    }
}
