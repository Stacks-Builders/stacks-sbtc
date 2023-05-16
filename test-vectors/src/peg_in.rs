use bitcoin::{
    opcodes, script::Builder, OutPoint, Script, Sequence, Transaction, TxIn, TxOut, Witness,
};

use crate::utils::generate_test_vector;

pub fn generate_peg_in_test_vector() -> Transaction {
    let input = TxIn {
        previous_output: OutPoint::null(),
        script_sig: Script::empty().into(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };

    let input = vec![input];

    let stacks_address = [1u8; 20];
    let stacks_testnet_singlesig_address_version = [26];
    let current_peg_wallet = [4; 32];

    let output = generate_peg_in_output(
        stacks_address,
        current_peg_wallet,
        stacks_testnet_singlesig_address_version,
    );

    generate_test_vector(input, output)
}

fn generate_peg_in_output(
    stacks_address: [u8; 20],
    current_peg_wallet: [u8; 32],
    stacks_address_version: [u8; 1],
) -> Vec<TxOut> {
    let op_bytes = [105, 100, b'<'];
    let padding_bytes = [0; 55];

    let op_return_script = Builder::new()
        .push_opcode(opcodes::all::OP_RETURN)
        .push_slice::<[u8; 3]>(op_bytes)
        .push_slice::<[u8; 1]>(stacks_address_version)
        .push_slice::<[u8; 20]>(stacks_address)
        .push_slice::<[u8; 55]>(padding_bytes)
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
            value: 0,
            script_pubkey: peg_wallet_script,
        },
    ]
}
