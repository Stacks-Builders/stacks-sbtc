use bitcoin::{consensus::encode::serialize, Transaction, TxIn, TxOut};
use secp256k1::{ecdsa::RecoveryId, Message, SecretKey};

/// Creates a serialized hex string from a `Transaction` struct
pub fn serialize_tx(tx: Transaction) -> String {
    array_bytes::bytes2hex("", serialize(&tx))
}

/// Generates a Recovery Id and a Signature from a msg and a secret key
pub fn generate_signature(msg: Vec<u8>, secret_key: SecretKey) -> (RecoveryId, [u8; 64]) {
    let msg_hash = sha256::digest(msg.as_slice());
    let msg_hash_bytes = array_bytes::hex2bytes(msg_hash).unwrap();
    let msg_ecdsa = Message::from_slice(&msg_hash_bytes).unwrap();

    secp256k1::Secp256k1::new()
        .sign_ecdsa_recoverable(&msg_ecdsa, &secret_key)
        .serialize_compact()
}

/// Generates a test vector from a series of inputs and outputs
pub fn generate_test_vector(input: Vec<TxIn>, output: Vec<TxOut>) -> Transaction {
    bitcoin::Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input,
        output,
    }
}
