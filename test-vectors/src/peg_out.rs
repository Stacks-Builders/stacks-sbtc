use std::iter::{once, repeat};

use bitcoin::{
    opcodes,
    script::{Builder, PushBytes},
    OutPoint, Script, Sequence, Transaction, TxIn, TxOut, Witness,
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
    let recipient_address = [5; 20];
    let amount = 1337;
    let fulfillment_fee = 42;
    let dust_amount = 21;

    let output = generate_peg_out_request_output(
        amount,
        secret_key,
        peg_wallet_address,
        recipient_address,
        fulfillment_fee,
        dust_amount,
    );

    generate_test_vector(input, output)
}

pub fn generate_peg_out_request_reveal_test_vector() -> Transaction {
    // Arbitrary key, copy-pasted from src/chainstate/stacks/tests/accounting.rs
    let secret_key_hex = "42faca653724860da7a41bfcef7e6ba78db55146f6900de8cb2a9f760ffac70c01";
    let secret_key_vec = array_bytes::hex2bytes(secret_key_hex).unwrap();
    let secret_key = SecretKey::from_slice(&secret_key_vec[..32]).unwrap();

    let peg_wallet_address = [4; 32];
    let recipient_address = [5; 20];
    let amount = 1337_u64;
    let fulfillment_fee = 42;
    let dust_amount = 21;

    let mut input = TxIn {
        previous_output: OutPoint::null(),
        script_sig: Script::empty().into(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };

    let recipient_script = Builder::new()
        .push_opcode(opcodes::all::OP_DUP)
        .push_opcode(opcodes::all::OP_HASH160)
        .push_slice(recipient_address)
        .push_opcode(opcodes::all::OP_EQUAL)
        .push_opcode(opcodes::all::OP_CHECKSIGVERIFY)
        .into_script();

    let mut msg = amount.to_be_bytes().to_vec();
    msg.extend_from_slice(recipient_script.as_bytes());

    let (recovery_id, signature) = generate_signature(msg, secret_key);

    let data = peg_out_request_data(None, amount, recovery_id.to_i32() as u8, signature, None);
    let data_as_push_bytes: &PushBytes = data.as_slice().try_into().unwrap();

    let witness_script = Builder::new()
        .push_slice(data_as_push_bytes)
        .push_opcode(opcodes::all::OP_DROP)
        .push_opcode(opcodes::all::OP_DUP)
        .push_opcode(opcodes::all::OP_HASH160)
        .push_slice(peg_wallet_address)
        .push_opcode(opcodes::all::OP_EQUAL)
        .push_opcode(opcodes::all::OP_CHECKSIGVERIFY)
        .into_script();

    let witness = vec![witness_script.as_bytes().to_vec(), [60; 97].to_vec()];

    input.witness = Witness::from_slice(&witness);

    let op_bytes = [105, 100, b'w'];

    let op_return_script = Builder::new()
        .push_opcode(opcodes::all::OP_RETURN)
        .push_slice(op_bytes)
        .into_script();

    let peg_wallet_script = Builder::new()
        .push_int(1)
        .push_slice(peg_wallet_address)
        .into_script();

    let output = vec![
        TxOut {
            value: 0,
            script_pubkey: op_return_script,
        },
        TxOut {
            value: dust_amount,
            script_pubkey: recipient_script.clone(),
        },
        TxOut {
            value: fulfillment_fee,
            script_pubkey: peg_wallet_script,
        },
    ];

    generate_test_vector(vec![input], output)
}

fn generate_peg_out_request_output(
    amount: u64,
    secret_key: SecretKey,
    peg_wallet_address: [u8; 32],
    recipient_address: [u8; 20],
    fulfillment_fee: u64,
    dust_amount: u64,
) -> Vec<TxOut> {
    let recipient_script = Builder::new()
        .push_opcode(opcodes::all::OP_DUP)
        .push_opcode(opcodes::all::OP_HASH160)
        .push_slice(recipient_address)
        .push_opcode(opcodes::all::OP_EQUAL)
        .push_opcode(opcodes::all::OP_CHECKSIGVERIFY)
        .into_script();

    let mut msg = amount.to_be_bytes().to_vec();
    msg.extend_from_slice(recipient_script.as_bytes());

    let op_bytes = [105, 100];
    let (recovery_id, signature) = generate_signature(msg, secret_key);

    let data = peg_out_request_data(
        op_bytes,
        amount,
        recovery_id.to_i32() as u8,
        signature,
        None,
    );
    let data_as_push_bytes: &PushBytes = data.as_slice().try_into().unwrap();

    let op_return_script = Builder::new()
        .push_opcode(opcodes::all::OP_RETURN)
        .push_slice(data_as_push_bytes)
        .into_script();

    let peg_wallet_script = Builder::new()
        .push_int(1)
        .push_slice(peg_wallet_address)
        .into_script();

    vec![
        TxOut {
            value: 0,
            script_pubkey: op_return_script,
        },
        TxOut {
            value: dust_amount,
            script_pubkey: recipient_script,
        },
        TxOut {
            value: fulfillment_fee,
            script_pubkey: peg_wallet_script,
        },
    ]
}

fn peg_out_request_data(
    magic_bytes: impl IntoIterator<Item = u8>,
    amount: u64,
    recovery_id: u8,
    signature_bytes: [u8; 64],
    maybe_fee_subsidy: Option<u64>,
) -> Vec<u8> {
    magic_bytes
        .into_iter()
        .chain(once(b'>'))
        .chain(amount.to_be_bytes())
        .chain(once(recovery_id))
        .chain(signature_bytes)
        .chain(repeat(0))
        .take(78)
        .chain(
            maybe_fee_subsidy
                .map(|fee_subsidy| fee_subsidy.to_be_bytes().to_vec())
                .into_iter()
                .flatten(),
        )
        .collect()
}
