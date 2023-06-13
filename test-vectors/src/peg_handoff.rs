use bitcoin::{
    opcodes, script::Builder, OutPoint, Script, Sequence, Transaction, TxIn, TxOut, Witness,
};

use crate::utils::generate_test_vector;

pub fn generate_peg_handoff_test_vector() -> Transaction {
    let input = TxIn {
        previous_output: OutPoint::null(),
        script_sig: Script::empty().into(),
        sequence: Sequence(65535),
        witness: Witness::new(),
    };

    let input = vec![input];

    let peg_wallet_address = [4; 32];
    let reward_cycle = 67;

    let output = generate_peg_handoff_output(peg_wallet_address, reward_cycle);

    generate_test_vector(input, output)
}

fn generate_peg_handoff_output(peg_wallet_address: [u8; 32], reward_cycle: u64) -> Vec<TxOut> {
    let op_bytes = [105, 100, b'H'];
    let padding_bytes = [0; 68];

    let op_return_script = Builder::new()
        .push_opcode(opcodes::all::OP_RETURN)
        .push_slice::<[u8; 3]>(op_bytes)
        .push_slice::<[u8; 8]>(reward_cycle.to_be_bytes())
        .push_slice::<[u8; 68]>(padding_bytes)
        .into_script();

    let peg_wallet_script = Builder::new()
        .push_int(1)
        .push_slice::<[u8; 32]>(peg_wallet_address)
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
