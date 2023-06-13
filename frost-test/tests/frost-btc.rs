use bitcoin::consensus::{Decodable, Encodable};
use bitcoin::secp256k1::Message;
use bitcoin::{EcdsaSighashType, OutPoint};
use test_utils::{
    build_transaction_deposit, build_transaction_withdrawal, generate_wallet, get_raw_transaction,
    mine_and_get_coinbase_txid, sign_transaction_ecdsa, sign_transaction_taproot, BitcoinProcess,
    SignerHelper,
};

#[test]
fn blog_post() {
    // https://medium.com/coinmonks/creating-and-signing-a-segwit-transaction-from-scratch-ec98577b526a
    let secp = bitcoin::secp256k1::Secp256k1::new();

    let secret_bytes =
        hex::decode("26F85CE8B2C635AD92F6148E4443FE415F512F3F29F44AB0E2CBDA819295BBD5").unwrap();
    let secret_key = bitcoin::secp256k1::SecretKey::from_slice(&secret_bytes).unwrap();
    let secp_public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let private_key = bitcoin::PrivateKey::new(secret_key, bitcoin::Network::Testnet);
    let public_key = bitcoin::PublicKey::from_private_key(&secp, &private_key);
    let address = bitcoin::Address::p2wpkh(&public_key, bitcoin::Network::Testnet).unwrap();
    println!(
        "address {} public_key {} {}",
        address,
        public_key,
        address.script_pubkey()
    );

    let blog_tx_bytes = hex::decode("02000000000103ed204affc7519dfce341db0569687569d12b1520a91a9824531c038ad62aa9d1010000006a47304402200da2c4d8f2f44a8154fe127fe5bbe93be492aa589870fe77eb537681bc29c8ec02201eee7504e37db2ef27fa29afda46b6c331cd1a651bb6fa5fd85dcf51ac01567a01210242BF11B788DDFF450C791F16E83465CC67328CA945C703469A08E37EF0D0E061ffffffff9cb872539fbe1bc0b9c5562195095f3f35e6e13919259956c6263c9bd53b20b70100000000ffffffff8012f1ec8aa9a63cf8b200c25ddae2dece42a2495cc473c1758972cfcd84d90401000000171600146a721dcca372f3c17b2c649b2ba61aa0fda98a91ffffffff01b580f50000000000160014cb61ee4568082cb59ac26bb96ec8fbe0109a4c000002483045022100f8dac321b0429798df2952d086e763dd5b374d031c7f400d92370ae3c5f57afd0220531207b28b1b137573941c7b3cf5384a3658ef5fc238d26150d8f75b2bcc61e70121025972A1F2532B44348501075075B31EB21C02EEF276B91DB99D30703F2081B7730247304402204ebf033caf3a1a210623e98b49acb41db2220c531843106d5c50736b144b15aa02201a006be1ebc2ffef0927d4458e3bb5e41e5abc7e44fc5ceb920049b46f879711012102AE68D299CBB8AB99BF24C9AF79A7B13D28AC8CD21F6F7F750300EDA41A589A5D00000000").unwrap();
    let transaction =
        bitcoin::Transaction::consensus_decode(&mut blog_tx_bytes.as_slice()).unwrap();
    println!("Blog Post tx {:?}", transaction);

    let mut transaction_bytes = vec![];
    transaction
        .consensus_encode(&mut transaction_bytes)
        .unwrap();
    assert_eq!(blog_tx_bytes, transaction_bytes);
    println!(
        "tx.input[1].witness ({} rows) {} {}",
        transaction.input[1].witness.len(),
        hex::encode(transaction.input[1].witness.second_to_last().unwrap()),
        hex::encode(transaction.input[1].witness.last().unwrap())
    );

    let segwit_signing_input_script_pubkey = address.script_pubkey().p2wpkh_script_code().unwrap();

    println!(
        "sighash input #{} script_pubkey {} value {}",
        1, &segwit_signing_input_script_pubkey, 9300
    );

    let mut comp = bitcoin::util::sighash::SighashCache::new(&transaction);
    let segwit_sighash = comp
        .segwit_signature_hash(
            1,
            &segwit_signing_input_script_pubkey,
            9300,
            EcdsaSighashType::All,
        )
        .unwrap();
    println!(
        "calc sighash len {} {}",
        segwit_sighash.len(),
        hex::encode(segwit_sighash.to_vec())
    );
    let blog_sighash_bytes =
        hex::decode("4876161197833dd58a1a2ba20728633677f38b9a7513a4d7d3714a7f7d3a1fa2").unwrap();
    println!(
        "blog sighash len {} {}",
        blog_sighash_bytes.len(),
        hex::encode(&blog_sighash_bytes)
    ); // second sha256
    assert_eq!(segwit_sighash.to_vec(), blog_sighash_bytes);

    let user_utxo_msg = Message::from_slice(&segwit_sighash).unwrap();
    let user_utxo_segwit_sig = secp.sign_ecdsa_low_r(&user_utxo_msg, &secret_key);
    let user_utxo_segwit_sig_bytes = user_utxo_segwit_sig.serialize_der();
    println!(
        "CALC SIG ({}) {}",
        user_utxo_segwit_sig_bytes.len(),
        hex::encode(&user_utxo_segwit_sig_bytes)
    );
    let calc_verify = secp.verify_ecdsa(&user_utxo_msg, &user_utxo_segwit_sig, &secp_public_key);
    assert!(calc_verify.is_ok(), "calc sig check {:?}", calc_verify);

    // libsecp verify only works on "low_r" 70 byte signatures
    // while this doesnt match the blog post, it is a sig of the same data, re-running openssl unil the result is short/low
    let blog_post_good_sig_bytes = hex::decode("30440220492eae58ddf8c2f8f1ab5b2b2c45432902a3c2dda508bf79319b3fde26e1364a022078bbdde1b79410efc07b19a64038242525883a94de3079668308aa45b035a6d8").unwrap();
    println!(
        "BLOG SIG ({}) {}",
        blog_post_good_sig_bytes.len(),
        hex::encode(&blog_post_good_sig_bytes)
    );
    let blog_sig =
        bitcoin::secp256k1::ecdsa::Signature::from_der(&blog_post_good_sig_bytes).unwrap();
    let blog_verify = secp.verify_ecdsa(&user_utxo_msg, &blog_sig, &secp_public_key);
    // https://docs.rs/secp256k1/0.24.1/src/secp256k1/ecdsa/mod.rs.html#400
    assert!(blog_verify.is_ok(), "blog sig check {:?}", blog_verify);
}

#[test]
fn frost_btc() {
    // Singer setup
    let mut signer = SignerHelper::default();
    // DKG (Distributed Key Generation)
    let (public_commitments, deposit_wallet_public_key_point, deposit_wallet_public_key) =
        signer.run_distributed_key_generation();

    // bitcoind regtest
    let btcd = BitcoinProcess::new();

    // create user source transaction keys
    let (source_secret_key, _, source_public_key, _, source_address, secp) = generate_wallet(false);
    // mine block to create btc
    let (source_txid, blockhash) = mine_and_get_coinbase_txid(&btcd, &source_address);
    let source_tx = get_raw_transaction(&btcd, &source_txid, Some(blockhash)).unwrap();

    // Deposit into a stx address
    let stx_address: [u8; 32] = [0; 32];
    println!("funding tx {:?}", source_tx);

    let source_utxo = &source_tx.output[0];
    println!(
        "funding UTXO with {:?} sats utxo.script_pub_key: {}",
        source_utxo.value,
        source_utxo.script_pubkey.asm()
    );
    // Use the unspent transaction from source mint transaction to build a deposit transaction
    let source_utxo_point = OutPoint {
        txid: source_tx.txid(),
        vout: 0,
    };

    let mut deposit_tx = build_transaction_deposit(
        source_utxo.value - 1000,
        deposit_wallet_public_key,
        stx_address,
        source_utxo_point,
    );
    let deposit_bytes_hex = sign_transaction_ecdsa(
        &source_address,
        &source_secret_key,
        &source_public_key,
        &source_tx.output[0],
        &mut deposit_tx,
        &secp,
    );

    let mut consensus_check_funding_out0: Vec<u8> = vec![];
    deposit_tx.output[0]
        .script_pubkey
        .consensus_encode(&mut consensus_check_funding_out0)
        .unwrap();

    println!(
        "deposit tx id {} outputs {:?}",
        deposit_tx.txid(),
        deposit_tx
            .output
            .iter()
            .map(|o| { o.value })
            .collect::<Vec<_>>()
    );

    let _ = btcd.rpc("decoderawtransaction", [&deposit_bytes_hex]);
    println!("deposit tx bytes {}", deposit_bytes_hex);
    let deposit_result_value = btcd.rpc("sendrawtransaction", [&deposit_bytes_hex]);
    assert!(deposit_result_value.is_string(), "{}", deposit_result_value);

    let deposit_utxo = &deposit_tx.output[1];
    // Peg out to btc address
    let deposit_utxo_point = OutPoint {
        txid: deposit_tx.txid(),
        vout: 1,
    };
    let withdrawal_amount = deposit_utxo.value - 2000;
    let mut withdrawal_tx =
        build_transaction_withdrawal(withdrawal_amount, source_public_key, deposit_utxo_point);

    let withdrawal_bytes_hex = sign_transaction_taproot(
        &mut withdrawal_tx,
        &deposit_utxo,
        &mut signer,
        &deposit_wallet_public_key_point,
        public_commitments,
    );
    println!(
        "withdrawal tx id {} outputs {:?}",
        withdrawal_tx.txid(),
        withdrawal_tx
            .output
            .iter()
            .map(|o| { o.value })
            .collect::<Vec<_>>()
    );

    println!("withdrawal tx bytes {}", &withdrawal_bytes_hex);

    let withdrawal_result_value = btcd.rpc("sendrawtransaction", [&withdrawal_bytes_hex]);
    assert!(
        withdrawal_result_value.is_string(),
        "{}",
        withdrawal_result_value
    );
}
