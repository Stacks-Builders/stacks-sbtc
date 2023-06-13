use std::iter::{once, repeat};

use bitcoin::{
    opcodes,
    script::{Builder, PushBytes},
    OutPoint, Script, Sequence, Transaction, TxIn, TxOut, Witness,
};

use crate::utils::generate_test_vector;

pub const C32_ADDRESS_VERSION_TESTNET_SINGLESIG: u8 = 26; // T

pub fn generate_peg_in_test_vector() -> Transaction {
    let input = TxIn {
        previous_output: OutPoint::null(),
        script_sig: Script::empty().into(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };

    let input = vec![input];

    let stacks_address = [1u8; 20];
    let current_peg_wallet = [4; 32];
    let amount = 1337;

    let output = generate_peg_in_output(
        stacks_address,
        current_peg_wallet,
        C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
        amount,
    );

    generate_test_vector(input, output)
}

/// Peg in reveal test vector with a taproot witness
pub fn generate_peg_in_reveal_test_vector() -> Transaction {
    let mut input = TxIn {
        previous_output: OutPoint::null(),
        script_sig: Script::empty().into(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };

    let stacks_address = [1u8; 20];
    let current_peg_wallet = [4; 32];
    let amount = 1337;

    let data = peg_in_data(
        None,
        stacks_address,
        C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
        Some("sbtc-receiver-contract".to_string()),
        None,
    );
    let data_as_push_bytes: &PushBytes = data.as_slice().try_into().unwrap();

    let witness_script = Builder::new()
        .push_slice(data_as_push_bytes)
        .push_opcode(opcodes::all::OP_DROP)
        .push_opcode(opcodes::all::OP_DUP)
        .push_opcode(opcodes::all::OP_HASH160)
        .push_slice(current_peg_wallet)
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
        .push_slice(current_peg_wallet)
        .into_script();

    let output = vec![
        TxOut {
            value: 0,
            script_pubkey: op_return_script,
        },
        TxOut {
            value: amount,
            script_pubkey: peg_wallet_script,
        },
    ];

    generate_test_vector(vec![input], output)
}

fn generate_peg_in_output(
    stacks_address: [u8; 20],
    current_peg_wallet: [u8; 32],
    stacks_address_version: u8,
    amount: u64,
) -> Vec<TxOut> {
    let op_bytes = [105, 100];

    let data = peg_in_data(
        op_bytes,
        stacks_address,
        stacks_address_version,
        Some("sbtc-receiver-contract".to_string()),
        None,
    );
    let data_as_push_bytes: &PushBytes = data.as_slice().try_into().unwrap();

    let op_return_script = Builder::new()
        .push_opcode(opcodes::all::OP_RETURN)
        .push_slice(data_as_push_bytes)
        .into_script();

    let peg_wallet_script = Builder::new()
        .push_int(1)
        .push_slice(current_peg_wallet)
        .into_script();

    vec![
        TxOut {
            value: 0,
            script_pubkey: op_return_script,
        },
        TxOut {
            value: amount,
            script_pubkey: peg_wallet_script,
        },
    ]
}

fn peg_in_data(
    magic_bytes: impl IntoIterator<Item = u8>,
    stacks_address: [u8; 20],
    stacks_address_version: u8,
    contract_name: Option<String>,
    maybe_fee_subsidy: Option<u64>,
) -> Vec<u8> {
    let principal_byte: u8 = match contract_name {
        Some(_) => 0x06,
        None => 0x05,
    };

    magic_bytes
        .into_iter()
        .chain(once(b'<'))
        .chain(once(principal_byte))
        .chain(once(stacks_address_version))
        .chain(stacks_address)
        .chain(
            contract_name
                .map(|contract_name| {
                    once(contract_name.len() as u8).chain(contract_name.as_bytes().to_vec())
                })
                .into_iter()
                .flatten(),
        )
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
