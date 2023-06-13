use std::str::FromStr;

use bitcoin::{Address, Network, OutPoint};
use stacks_coordinator::{
    bitcoin_node::{BitcoinNode, LocalhostBitcoinNode},
    bitcoin_wallet::BitcoinWallet,
    peg_wallet::BitcoinWallet as BitcoinWalletTrait,
};
use test_utils::{
    build_transaction_deposit, build_transaction_withdrawal, generate_wallet, get_raw_transaction,
    mine_and_get_coinbase_txid, sign_transaction_ecdsa, sign_transaction_taproot, BitcoinProcess,
    SignerHelper,
};

#[test]
fn should_broadcast_transaction() {
    let btcd = BitcoinProcess::new();
    let local_btc_node = LocalhostBitcoinNode::new(btcd.url().clone());

    // Generates a source wallet
    let (source_secret_key, _, source_public_key, _, source_address, secp) = generate_wallet(false);
    // Mint some funds to the source wallet
    let (source_txid, blockhash) = mine_and_get_coinbase_txid(&btcd, &source_address);
    // Get the coinbase transaction
    let source_tx = get_raw_transaction(&btcd, &source_txid, Some(blockhash)).unwrap();

    // Get a deposit escrow wallet public key i.e. address
    let mut signer = SignerHelper::default();
    let (public_commitments, deposit_wallet_public_key_point, deposit_wallet_public_key) =
        signer.run_distributed_key_generation();

    // Deposit into a stx address
    let stx_address: [u8; 32] = [0; 32];
    // Use the unspent transaction from source mint transaction to build a deposit transaction
    let source_utxo_outpoint = OutPoint {
        txid: source_tx.txid(),
        vout: 0,
    };

    let source_utxo = &source_tx.output[0];

    let mut deposit_tx = build_transaction_deposit(
        source_tx.output[0].value,
        deposit_wallet_public_key,
        stx_address,
        source_utxo_outpoint,
    );

    // Pay 1 sat/vbyte fee
    // Convert from BTC to Sats: (1 BTC = 100,000,000 Satoshis)
    let fee = btcd
        .rpc("getmempoolinfo", ())
        .get("minrelaytxfee")
        .unwrap()
        .as_f64()
        .map(|fee| fee * 100_000_000.0)
        .unwrap() as u64;

    // Set correct sat amounts
    deposit_tx.output[1].value = source_utxo.value - fee as u64;

    // Sign the transaction
    sign_transaction_ecdsa(
        &source_address,
        &source_secret_key,
        &source_public_key,
        source_utxo,
        &mut deposit_tx,
        &secp,
    );

    // Attempt to broadcast the deposit transaction
    let deposit_txid = local_btc_node.broadcast_transaction(&deposit_tx).unwrap();
    // Ensure it was successfully broadcast
    assert!(get_raw_transaction(&btcd, &deposit_txid, None).is_ok());

    // Withdraw to user's BTC address
    let deposit_utxo_point = OutPoint {
        txid: deposit_tx.txid(),
        vout: 1,
    };
    let deposit_utxo = &deposit_tx.output[1];
    let amount_to_withdraw = deposit_utxo.value - 1000 - fee as u64; //amount previously deposited minus some value
    let mut withdrawal_tx =
        build_transaction_withdrawal(amount_to_withdraw, source_public_key, deposit_utxo_point);

    // Use the unspent output from the deposit transaction to build a withdrawal transaction
    sign_transaction_taproot(
        &mut withdrawal_tx,
        deposit_utxo,
        &mut signer,
        &deposit_wallet_public_key_point,
        public_commitments,
    );

    // Attempt to broadcast the withdrawal transaction
    let withdrawal_txid = local_btc_node
        .broadcast_transaction(&withdrawal_tx)
        .unwrap();
    // Ensure it was broadcast correctly
    assert!(get_raw_transaction(&btcd, &withdrawal_txid, None).is_ok());
}

#[test]
fn should_load_wallet() {
    let btcd = BitcoinProcess::new();
    let (_, _, _, xonly_pubkey, address, _) = generate_wallet(true);
    dbg!("address: {}", &address);
    let wallet = BitcoinWallet::new(xonly_pubkey, Network::Regtest);

    // Attemp to register the address with the wallet
    let local_btc_node = LocalhostBitcoinNode::new(btcd.url().clone());
    local_btc_node.load_wallet(&wallet).unwrap();
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
                .map(|addr| {
                    dbg!(&addr);
                    dbg!(&address);
                    addr == address
                })
                .unwrap_or_default()
        })
        .is_some();

    assert!(address_found);
}

#[test]
fn should_list_unspent() {
    let btcd = BitcoinProcess::new();

    let (_, _, _, xonly_pubkey, _, _) = generate_wallet(true);

    let wallet = BitcoinWallet::new(xonly_pubkey, Network::Regtest);

    let local_btc_node = LocalhostBitcoinNode::new(btcd.url().clone());
    local_btc_node.load_wallet(&wallet).unwrap();

    // Produce some UTXOs for the address
    let _ = mine_and_get_coinbase_txid(&btcd, wallet.address());
    // Produce more blocks to make sure the UTXOs are confirmed
    let _ = mine_and_get_coinbase_txid(&btcd, &wallet.address());

    let utxos = local_btc_node.list_unspent(&wallet).unwrap();

    assert!(!utxos.is_empty());
}
