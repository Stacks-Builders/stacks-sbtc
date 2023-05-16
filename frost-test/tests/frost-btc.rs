use bitcoin::consensus::{Decodable, Encodable};
use bitcoin::psbt::serialize::Serialize;
use bitcoin::schnorr::TweakedPublicKey;
use bitcoin::secp256k1::{rand, Message};
use bitcoin::{
    EcdsaSighashType, OutPoint, PackedLockTime, SchnorrSighashType, Script, Transaction,
    XOnlyPublicKey,
};
use rand_core::OsRng;
use test_utils::{mine_and_get_coinbase_txid, BitcoinProcess};
use wsts::common::PolyCommitment;
use wsts::{
    bip340::{
        test_helpers::{dkg, sign},
        Error as Bip340Error, SchnorrProof,
    },
    v1::{self, SignatureAggregator},
    Point,
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
    let threshold = 3;
    let total = 4;
    let mut rng = OsRng::default();
    let mut signers = [
        v1::Signer::new(1, &[0, 1], total, threshold, &mut rng),
        v1::Signer::new(2, &[2], total, threshold, &mut rng),
        v1::Signer::new(3, &[3], total, threshold, &mut rng),
    ];
    let secp = bitcoin::secp256k1::Secp256k1::new();

    // DKG (Distributed Key Generation)
    let (public_key_shares, group_public_key) = dkg_round(&mut rng, &mut signers);

    // Peg Wallet Address from group key
    let peg_wallet_address =
        bitcoin::PublicKey::from_slice(&group_public_key.compress().as_bytes()).unwrap();

    // bitcoind regtest
    let btcd = BitcoinProcess::new();

    // create user keys
    let user_secret_key = bitcoin::secp256k1::SecretKey::new(&mut rand::thread_rng());
    let user_secp_public_key =
        bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &user_secret_key);
    let user_private_key = bitcoin::PrivateKey::new(user_secret_key, bitcoin::Network::Regtest);
    let user_public_key = bitcoin::PublicKey::from_private_key(&secp, &user_private_key);
    let user_address =
        bitcoin::Address::p2wpkh(&user_public_key, bitcoin::Network::Regtest).unwrap();
    println!("user private key {}", user_private_key);
    println!(
        "user address {} public key {} witness hash {:?} p2wpkh signing script {}",
        user_address,
        hex::encode(user_public_key.serialize()),
        user_public_key.wpubkey_hash().unwrap(),
        user_address.script_pubkey().p2wpkh_script_code().unwrap()
    );

    // mine block to create btc
    let (txid, block_id) = mine_and_get_coinbase_txid(&btcd, &user_address);
    println!("mined block_id {:?}", block_id);
    println!("mined txid {:?}", txid);
    let result = btcd.rpc("getrawtransaction", (txid.to_string(), false, block_id));
    let user_funding_transaction_bytes_hex = result.as_str().unwrap();
    let _ = btcd.rpc(
        "decoderawtransaction",
        [&user_funding_transaction_bytes_hex],
    );

    // Peg in to stx address
    let stx_address = [0; 32];
    let user_funding_transaction = bitcoin::Transaction::consensus_decode(
        &mut hex::decode(user_funding_transaction_bytes_hex)
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    println!(
        "funding tx txid {} wtxid {}",
        user_funding_transaction.txid(),
        user_funding_transaction.wtxid()
    );
    println!("funding tx {:?}", user_funding_transaction);

    let funding_utxo = &user_funding_transaction.output[0];
    println!(
        "funding UTXO with {:?} sats utxo.script_pub_key: {}",
        funding_utxo.value,
        funding_utxo.script_pubkey.asm()
    );
    let mut peg_in = build_peg_in_op_return(
        funding_utxo.value - 1000,
        peg_wallet_address,
        stx_address,
        &user_funding_transaction,
        0,
    );
    let peg_in_sighash_pubkey_script = user_address.script_pubkey().p2wpkh_script_code().unwrap();
    let mut sighash_cache_peg_in = bitcoin::util::sighash::SighashCache::new(&peg_in);
    let peg_in_sighash = sighash_cache_peg_in
        .segwit_signature_hash(
            0,
            &peg_in_sighash_pubkey_script,
            funding_utxo.value,
            EcdsaSighashType::All,
        )
        .unwrap();
    println!("peg-in segwit sighash {}", peg_in_sighash);
    let peg_in_msg = Message::from_slice(&peg_in_sighash).unwrap();
    let peg_in_sig = secp.sign_ecdsa_low_r(&peg_in_msg, &user_secret_key);
    let peg_in_verify = secp.verify_ecdsa(&peg_in_msg, &peg_in_sig, &user_secp_public_key);
    assert!(peg_in_verify.is_ok());
    //let (peg_in_step_a, peg_in_step_b) = two_phase_peg_in(peg_wallet_address, stx_address, user_utxo);
    peg_in.input[0]
        .witness
        .push_bitcoin_signature(&peg_in_sig.serialize_der(), EcdsaSighashType::All);
    peg_in.input[0].witness.push(user_public_key.serialize());
    let mut peg_in_bytes: Vec<u8> = vec![];
    peg_in.consensus_encode(&mut peg_in_bytes).unwrap();

    let mut consensus_check_funding_out0: Vec<u8> = vec![];
    user_funding_transaction.output[0]
        .script_pubkey
        .consensus_encode(&mut consensus_check_funding_out0)
        .unwrap();

    println!(
        "peg-in tx id {} outputs {:?}",
        peg_in.txid(),
        peg_in
            .output
            .iter()
            .map(|o| { o.value })
            .collect::<Vec<_>>()
    );
    let peg_in_bytes_hex = hex::encode(&peg_in_bytes);
    let _ = btcd.rpc("decoderawtransaction", [&peg_in_bytes_hex]);
    println!("peg-in tx bytes {}", peg_in_bytes_hex);
    let peg_in_result_value = btcd.rpc("sendrawtransaction", [&peg_in_bytes_hex]);
    assert!(peg_in_result_value.is_string(), "{}", peg_in_result_value);

    let peg_in_utxo = &peg_in.output[1];
    // Peg out to btc address
    let peg_in_utxo_point = OutPoint {
        txid: peg_in.txid(),
        vout: 1,
    };
    let mut peg_out = build_peg_out(peg_in_utxo.value - 2000, user_public_key, peg_in_utxo_point);
    let mut sighash_cache_peg_out = bitcoin::util::sighash::SighashCache::new(&peg_out);
    let taproot_sighash = sighash_cache_peg_out
        .taproot_key_spend_signature_hash(
            0,
            &bitcoin::util::sighash::Prevouts::All(&[&peg_in.output[1]]),
            SchnorrSighashType::Default,
        )
        .unwrap();
    println!("peg-out taproot sighash {}", hex::encode(taproot_sighash),);
    let signing_payload = taproot_sighash.as_hash().to_vec();
    // signing. Signers: 0 (parties: 0, 1) and 1 (parties: 2)
    let schnorr_proof = signing_round(
        &signing_payload,
        threshold,
        total,
        &mut rng,
        &mut signers,
        public_key_shares,
    )
    .unwrap();
    assert!(schnorr_proof.verify(&group_public_key.x(), &signing_payload));

    let _taproot_sighash_msg = Message::from_slice(&taproot_sighash).unwrap();
    let mut frost_sig_bytes = vec![];
    frost_sig_bytes.extend(schnorr_proof.r.to_bytes());
    frost_sig_bytes.extend(schnorr_proof.s.to_bytes());
    // is &group_public_key.x().to_bytes() used?

    peg_out.input[0].witness.push(&frost_sig_bytes);
    println!(
        "frost sig ({}) {}",
        frost_sig_bytes.len(),
        hex::encode(frost_sig_bytes)
    );
    println!(
        "peg-out tx id {} outputs {:?}",
        peg_out.txid(),
        peg_out
            .output
            .iter()
            .map(|o| { o.value })
            .collect::<Vec<_>>()
    );

    let mut peg_out_bytes: Vec<u8> = vec![];
    let _peg_out_bytes_len = peg_out.consensus_encode(&mut peg_out_bytes).unwrap();
    let peg_out_bytes_hex = hex::encode(&peg_out_bytes);

    println!("peg-out tx bytes {}", &peg_out_bytes_hex);

    let peg_out_result_value = btcd.rpc("sendrawtransaction", [&peg_out_bytes_hex]);
    assert!(peg_out_result_value.is_string(), "{}", peg_out_result_value);
}

fn build_peg_in_op_return(
    satoshis: u64,
    peg_wallet_address: bitcoin::PublicKey,
    stx_address: [u8; 32],
    utxo: &Transaction,
    utxo_vout: u32,
) -> Transaction {
    let utxo_point = OutPoint {
        txid: utxo.txid(),
        vout: utxo_vout,
    };
    let witness = bitcoin::blockdata::witness::Witness::new();
    let peg_in_input = bitcoin::TxIn {
        previous_output: utxo_point,
        script_sig: Default::default(),
        sequence: bitcoin::Sequence(0xFFFFFFFF),
        witness: witness,
    };
    let mut sip_21_peg_in_data = vec![0, 0, '<' as u8];
    sip_21_peg_in_data.extend_from_slice(&stx_address);
    let op_return = Script::new_op_return(&sip_21_peg_in_data);
    let peg_in_output_0 = bitcoin::TxOut {
        value: 0,
        script_pubkey: op_return,
    };
    let _secp = bitcoin::util::key::Secp256k1::new();
    // crate type weirdness
    let peg_wallet_address_secp =
        bitcoin::secp256k1::PublicKey::from_slice(&peg_wallet_address.to_bytes()).unwrap();
    let peg_wallet_address_xonly = XOnlyPublicKey::from(peg_wallet_address_secp);
    //let peg_wallet_address_tweaked = peg_wallet_address_xonly.tap_tweak(&secp, None);
    let peg_wallet_address_tweaked =
        TweakedPublicKey::dangerous_assume_tweaked(peg_wallet_address_xonly);
    println!(
        "build peg-in with shared wallet public key {} tweaked {:?}",
        peg_wallet_address_secp, peg_wallet_address_tweaked
    );
    let taproot = Script::new_v1_p2tr_tweaked(peg_wallet_address_tweaked);
    let peg_in_output_1 = bitcoin::TxOut {
        value: satoshis,
        script_pubkey: taproot,
    };
    bitcoin::blockdata::transaction::Transaction {
        version: 2,
        lock_time: PackedLockTime(0),
        input: vec![peg_in_input],
        output: vec![peg_in_output_0, peg_in_output_1],
    }
}

fn build_peg_out(satoshis: u64, user_address: bitcoin::PublicKey, utxo: OutPoint) -> Transaction {
    let peg_out_input = bitcoin::TxIn {
        previous_output: utxo,
        script_sig: Default::default(),
        sequence: Default::default(),
        witness: Default::default(),
    };
    let p2wpk = Script::new_v0_p2wpkh(&user_address.wpubkey_hash().unwrap());
    let peg_out_output = bitcoin::TxOut {
        value: satoshis,
        script_pubkey: p2wpk,
    };
    bitcoin::blockdata::transaction::Transaction {
        version: 2,
        lock_time: PackedLockTime(0),
        input: vec![peg_out_input],
        output: vec![peg_out_output],
    }
}

fn signing_round(
    message: &[u8],
    threshold: u32,
    total: u32,
    rng: &mut OsRng,
    signers: &mut [v1::Signer; 3],
    public_commitments: Vec<PolyCommitment>,
) -> Result<SchnorrProof, Bip340Error> {
    // decide which signers will be used
    let mut signers = [signers[0].clone(), signers[1].clone()];

    let (nonces, shares) = sign(message, &mut signers, rng);

    let sig = SignatureAggregator::new(total, threshold, public_commitments.clone())
        .unwrap()
        .sign(&message, &nonces, &shares)
        .unwrap();

    SchnorrProof::new(&sig)
}

fn dkg_round(rng: &mut OsRng, signers: &mut [v1::Signer; 3]) -> (Vec<PolyCommitment>, wsts::Point) {
    let polys = dkg(signers, rng).unwrap();
    let pubkey = polys.iter().fold(Point::new(), |s, poly| s + poly.A[0]);
    (polys, pubkey)
}
