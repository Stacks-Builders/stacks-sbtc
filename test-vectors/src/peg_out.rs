use bitcoin::{
    opcodes, script::Builder, OutPoint, Script, Sequence, Transaction, TxIn, TxOut, Witness,
};
use secp256k1::SecretKey;

use crate::utils::{generate_signature, generate_test_vector};

pub fn generate_peg_out_request_test_vector() -> Transaction {
    let input = TxIn {
        previous_output: OutPoint::null(),
        script_sig: Script::empty().into(),
        sequence: Sequence(65535),
        witness: Witness::new(),
    };

    let input = vec![input];

    // Arbitrary key, copy-pasted from src/chainstate/stacks/tests/accounting.rs
    let secret_key_hex = "42faca653724860da7a41bfcef7e6ba78db55146f6900de8cb2a9f760ffac70c01";
    let secret_key_vec = array_bytes::hex2bytes(secret_key_hex).unwrap();
    let secret_key = SecretKey::from_slice(&secret_key_vec[..32]).unwrap();

    let peg_wallet_address = [4; 32];
    let amount = 1337;
    let fulfillment_fee = 42;

    let output =
        generate_peg_out_request_output(amount, secret_key, peg_wallet_address, fulfillment_fee);

    generate_test_vector(input, output)
}

fn generate_peg_out_request_output(
    amount: u64,
    secret_key: SecretKey,
    peg_wallet_address: [u8; 32],
    fulfillment_fee: u64,
) -> Vec<TxOut> {
    let p2tr_script = Builder::new()
        .push_int(1)
        .push_slice(peg_wallet_address)
        .into_script();

    let mut msg = amount.to_be_bytes().to_vec();
    msg.extend_from_slice(p2tr_script.as_bytes());

    let op_bytes = [105, 100, b'>'];
    let (recovery_id, signature) = generate_signature(msg, secret_key);
    let padding_bytes = [0; 3];

    let op_return_script = Builder::new()
        .push_opcode(opcodes::all::OP_RETURN)
        .push_slice::<[u8; 3]>(op_bytes)
        .push_slice::<[u8; 8]>(amount.to_be_bytes())
        .push_slice::<[u8; 1]>([recovery_id.to_i32() as u8])
        .push_slice::<[u8; 64]>(signature)
        .push_slice::<[u8; 3]>(padding_bytes)
        .into_script();

    vec![
        TxOut {
            value: 0,
            script_pubkey: op_return_script,
        },
        TxOut {
            value: amount,
            script_pubkey: p2tr_script.clone(),
        },
        TxOut {
            value: fulfillment_fee,
            script_pubkey: p2tr_script,
        },
    ]
}
