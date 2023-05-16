use std::str::FromStr;

use bitcoin::{
    consensus::Decodable, secp256k1::Secp256k1, Address, OutPoint, PackedLockTime, Script,
    Sequence, Transaction, TxIn, TxOut, Witness,
};
use stacks_coordinator::bitcoin_node::{BitcoinNode, LocalhostBitcoinNode};
use test_utils::{generate_wallet, mine_and_get_coinbase_txid, sign_transaction, BitcoinProcess};

#[test]
fn should_broadcast_transaction() {
    let secp = Secp256k1::new();
    let btcd = BitcoinProcess::new();

    // Generates source and destination wallets
    let (secret_key_source, _, public_key_source, address_source) = generate_wallet();
    let (_, _, _, address_destination) = generate_wallet();

    // Mint some funds to the source wallet
    let (txid, blockhash) = mine_and_get_coinbase_txid(&btcd, &address_source);

    // Get coinbase transaction
    let tx = {
        let txraw = btcd
            .rpc("getrawtransaction", (&txid.to_string(), false, blockhash))
            .as_str()
            .unwrap()
            .to_string();

        Transaction::consensus_decode(&mut hex::decode(txraw).unwrap().as_slice()).unwrap()
    };

    // Use coinbase output as input for new transaction
    let input = TxIn {
        previous_output: OutPoint::new(txid, 0),
        script_sig: Script::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };

    // Output that sends funds to the destination address
    let output1 = TxOut {
        value: 0,
        script_pubkey: address_destination.script_pubkey(),
    };
    // Output that sends funds back to the source address
    let output2 = TxOut {
        value: 0,
        script_pubkey: address_source.script_pubkey(),
    };

    let mut tx_new = Transaction {
        version: 1,
        lock_time: PackedLockTime::ZERO,
        input: vec![input],
        output: vec![output1, output2],
    };

    // Pay 1 sat/vbyte fee
    let fee = tx.vsize() * 1 + 1;
    let to_send = 1000;

    // Set correct sat amounts
    tx_new.output[0].value = to_send;
    tx_new.output[1].value = tx.output[0].value - to_send - fee as u64;

    sign_transaction(
        &address_source,
        &secret_key_source,
        &public_key_source,
        &tx.output[0],
        &mut tx_new,
        &secp,
    );

    let local_btc_node = LocalhostBitcoinNode::new(btcd.url().to_string());

    // Attempt to broadcast the transaction
    let txid = local_btc_node.broadcast_transaction(&tx_new).unwrap();

    // Check that the transaction was broadcasted
    let txraw = btcd
        .rpc("getrawtransaction", (&txid.to_string(), false))
        .as_str()
        .unwrap()
        .to_string();
    assert!(Transaction::consensus_decode(&mut hex::decode(txraw).unwrap().as_slice()).is_ok());
}

#[test]
fn should_load_wallet() {
    let btcd = BitcoinProcess::new();
    let (_, _, _, address) = generate_wallet();

    // Attemp to register the address with the wallet
    let local_btc_node = LocalhostBitcoinNode::new(btcd.url().to_string());
    local_btc_node.load_wallet(&address).unwrap();
    let result = btcd.rpc("listreceivedbyaddress", (0, true, true));

    // Check that the address was registered
    let address_found = result
        .as_array()
        .unwrap()
        .iter()
        .find(|item| {
            item.get("address")
                .and_then(|addr| addr.as_str())
                .and_then(|addr| Address::from_str(addr).ok())
                .map(|addr| addr == address)
                .unwrap_or_default()
        })
        .is_some();

    assert!(address_found);
}

#[test]
fn should_list_unspent() {
    let btcd = BitcoinProcess::new();
    let (_, _, _, address) = generate_wallet();

    let local_btc_node = LocalhostBitcoinNode::new(btcd.url().to_string());
    local_btc_node.load_wallet(&address).unwrap();

    // Produce some UTXOs for the address
    let _ = mine_and_get_coinbase_txid(&btcd, &address);
    // Produce more blocks to make sure the UTXOs are confirmed
    let _ = mine_and_get_coinbase_txid(&btcd, &address);

    let utxos = local_btc_node.list_unspent(&address).unwrap();

    assert!(!utxos.is_empty());
}
