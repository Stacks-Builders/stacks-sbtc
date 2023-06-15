use std::collections::{BTreeMap, HashMap};
use std::iter::{once, repeat};
use std::str::FromStr;

use anyhow::anyhow;
use bdk::blockchain::Blockchain;
use bdk::SignOptions;
use bdk::{
    blockchain::ElectrumBlockchain, database::MemoryDatabase, electrum_client::Client,
    template::P2Wpkh, SyncOptions, Wallet,
};
use bitcoin::blockdata::opcodes;
use bitcoin::blockdata::script::Builder;
use bitcoin::psbt::serialize::{Deserialize, Serialize};
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::{Address as BitcoinAddress, Network, PrivateKey, TxOut};
use bitcoin::{Script, Transaction};
use blockstack_lib::types::{chainstate::StacksAddress, Address};
use clap::Parser;

fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();

    match args.command {
        Command::Deposit(deposit_args) => build_deposit_tx(&deposit_args),
        Command::Withdraw(withdrawal_args) => build_withdrawal_tx(&withdrawal_args),
        Command::Broadcast(broadcast_args) => broadcast_tx(&broadcast_args),
    }
}

#[derive(clap::Subcommand, Debug, Clone)]
enum Command {
    Deposit(DepositArgs),
    Withdraw(WithdrawalArgs),
    Broadcast(BroadcastArgs),
}

#[derive(Parser, Debug, Clone)]
struct DepositArgs {
    /// P2WPKH private key in WIF format
    #[clap(short, long)]
    wif: String,

    /// Stacks address that will receive sBTC
    #[clap(short, long)]
    recipient: String,

    /// The amount of sats to send
    #[clap(short, long)]
    amount: u64,

    /// Dkg wallet address
    #[clap(short, long)]
    dkg_wallet: String,
}

#[derive(Parser, Debug, Clone)]
struct WithdrawalArgs {
    /// P2WPKH private key in WIF format
    #[clap(short, long)]
    wif: String,

    /// Bitcoin address that will receive BTC
    #[clap(short, long)]
    recipient: String,

    /// The amount of sats to send
    #[clap(short, long)]
    amount: u64,

    /// The amount of sats to send as the fulfillment fee
    #[clap(short, long)]
    fulfillment_fee: u64,

    /// Dkg wallet address
    #[clap(short, long)]
    dkg_wallet: String,
}

#[derive(Parser, Debug, Clone)]
struct BroadcastArgs {
    /// The network to broadcast to
    #[clap(short, long, default_value_t = Network::Testnet)]
    network: Network,
    /// The transaction to broadcast
    tx: String,
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn init_blockchain() -> anyhow::Result<ElectrumBlockchain> {
    let client = Client::new("ssl://blockstream.info:993")?;
    let blockchain = ElectrumBlockchain::from(client);
    Ok(blockchain)
}

fn build_deposit_tx(deposit: &DepositArgs) -> anyhow::Result<()> {
    let private_key = PrivateKey::from_wif(&deposit.wif)?;

    let wallet = setup_wallet(private_key)?;

    let recipient = StacksAddress::from_string(&deposit.recipient)
        .ok_or(anyhow::anyhow!("Could not parse recipient Stacks address"))?;
    let dkg_address = BitcoinAddress::from_str(&deposit.dkg_wallet)?;

    let mut psbt = deposit_psbt(
        &wallet,
        &recipient,
        &dkg_address,
        deposit.amount,
        &private_key.network,
    )?;

    wallet.sign(&mut psbt, SignOptions::default())?;
    let tx = psbt.extract_tx();
    println!("Resulting deposit txid: {}", tx.txid());
    println!(
        "Resulting serialized deposit tx:\n{}",
        array_bytes::bytes2hex("", tx.serialize())
    );

    Ok(())
}

fn build_withdrawal_tx(withdrawal: &WithdrawalArgs) -> anyhow::Result<()> {
    let private_key = PrivateKey::from_wif(&withdrawal.wif)?;

    let wallet = setup_wallet(private_key)?;

    let recipient = BitcoinAddress::from_str(&withdrawal.recipient)?;
    let dkg_address = BitcoinAddress::from_str(&withdrawal.dkg_wallet)?;

    let mut psbt = withdrawal_psbt(
        &wallet,
        &private_key,
        &recipient,
        &dkg_address,
        withdrawal.amount,
        withdrawal.fulfillment_fee,
        &private_key.network,
    )?;

    wallet.sign(&mut psbt, SignOptions::default())?;
    let tx = psbt.extract_tx();
    println!("Resulting withdrawal txid: {}", tx.txid());
    println!(
        "Resulting serialized withdrawal tx:\n{}",
        array_bytes::bytes2hex("", tx.serialize())
    );
    Ok(())
}

fn broadcast_tx(broadcast: &BroadcastArgs) -> anyhow::Result<()> {
    let blockchain = init_blockchain()?;
    let tx = Transaction::deserialize(
        &array_bytes::hex2bytes(&broadcast.tx).map_err(|e| anyhow!("{:?}", e))?,
    )?;
    blockchain.broadcast(&tx)?;

    println!("Broadcasted tx: {}", tx.txid());
    Ok(())
}

fn setup_wallet(private_key: PrivateKey) -> anyhow::Result<Wallet<MemoryDatabase>> {
    let blockchain = init_blockchain()?;
    let wallet = Wallet::new(
        P2Wpkh(private_key),
        Some(P2Wpkh(private_key)),
        private_key.network,
        MemoryDatabase::default(),
    )?;

    wallet.sync(&blockchain, SyncOptions::default())?;

    Ok(wallet)
}

fn deposit_psbt(
    wallet: &Wallet<MemoryDatabase>,
    recipient: &StacksAddress,
    dkg_address: &BitcoinAddress,
    amount: u64,
    network: &Network,
) -> anyhow::Result<PartiallySignedTransaction> {
    let mut tx_builder = wallet.build_tx();

    let op_return_script = build_op_return_script(&deposit_data(recipient, network));
    let dkg_script = dkg_address.script_pubkey();
    let dust_amount = dkg_script.dust_value().to_sat();

    if amount < dust_amount {
        return Err(anyhow!(
            "Provided amount {} is less than the dust amount: {}",
            amount,
            dust_amount
        ));
    }

    let outputs = [(op_return_script, 0), (dkg_script, amount)];

    for (script, amount) in outputs.clone() {
        tx_builder.add_recipient(script, amount);
    }

    let (mut partial_tx, _) = tx_builder.finish()?;

    partial_tx.unsigned_tx.output =
        reorder_outputs(partial_tx.unsigned_tx.output.into_iter(), outputs);

    Ok(partial_tx)
}

fn withdrawal_psbt(
    wallet: &Wallet<MemoryDatabase>,
    private_key: &PrivateKey,
    recipient: &BitcoinAddress,
    dkg_address: &BitcoinAddress,
    amount: u64,
    fulfillment_fee: u64,
    network: &Network,
) -> anyhow::Result<PartiallySignedTransaction> {
    let recipient_script = recipient.script_pubkey();
    let dkg_wallet_script = dkg_address.script_pubkey();

    // Check that we have enough to cover dust
    let recipient_dust_amount = recipient_script.dust_value().to_sat();
    let dkg_wallet_dust_amount = dkg_wallet_script.dust_value().to_sat();

    if fulfillment_fee < dkg_wallet_dust_amount {
        return Err(anyhow!(
            "Provided fulfillment fee {} is less than the dust amount: {}",
            fulfillment_fee,
            dkg_wallet_dust_amount
        ));
    }

    let op_return_script = build_op_return_script(&withdrawal_data(
        recipient,
        amount,
        fulfillment_fee,
        private_key,
        network,
    ));

    let mut tx_builder = wallet.build_tx();

    let outputs = [
        (op_return_script, 0),
        (recipient_script, recipient_dust_amount),
        (dkg_wallet_script, fulfillment_fee),
    ];

    for (script, amount) in outputs.clone() {
        tx_builder.add_recipient(script, amount);
    }

    let (mut partial_tx, _) = tx_builder.finish()?;

    partial_tx.unsigned_tx.output =
        reorder_outputs(partial_tx.unsigned_tx.output.into_iter(), outputs);

    Ok(partial_tx)
}

fn build_op_return_script(data: &[u8]) -> Script {
    Builder::new()
        .push_opcode(opcodes::all::OP_RETURN)
        .push_slice(data)
        .into_script()
}

fn withdrawal_data(
    recipient: &BitcoinAddress,
    amount: u64,
    fulfillment_fee: u64,
    private_key: &PrivateKey,
    network: &Network,
) -> Vec<u8> {
    let mut msg = amount.to_be_bytes().to_vec();
    msg.extend_from_slice(recipient.script_pubkey().as_bytes());

    let msg_hash = sha256::digest(msg.as_slice());
    let msg_hash_bytes = array_bytes::hex2bytes(msg_hash).unwrap();
    let msg_ecdsa = Message::from_slice(&msg_hash_bytes).unwrap();

    let (recovery_id, signature) = Secp256k1::new()
        .sign_ecdsa_recoverable(&msg_ecdsa, &private_key.inner)
        .serialize_compact();

    magic_bytes(network)
        .into_iter()
        .chain(once(b'>'))
        .chain(amount.to_be_bytes())
        .chain(once(recovery_id.to_i32() as u8))
        .chain(signature)
        .chain(repeat(0))
        .take(78)
        .chain(fulfillment_fee.to_be_bytes().to_vec())
        .collect()
}

fn deposit_data(recipient: &StacksAddress, network: &Network) -> Vec<u8> {
    magic_bytes(network)
        .into_iter()
        .chain(once(b'<'))
        .chain(once(recipient.version))
        .chain(recipient.to_bytes())
        .collect()
}

fn magic_bytes(network: &Network) -> [u8; 2] {
    match network {
        Network::Bitcoin => [b'X', b'2'],
        Network::Testnet => [b'T', b'2'],
        _ => [b'i', b'd'],
    }
}

fn reorder_outputs(
    outputs: impl IntoIterator<Item = TxOut>,
    order: impl IntoIterator<Item = (Script, u64)>,
) -> Vec<TxOut> {
    let indices: HashMap<(Script, u64), usize> = order
        .into_iter()
        .enumerate()
        .map(|(idx, val)| (val, idx))
        .collect();

    let outputs_ordered: BTreeMap<usize, TxOut> = outputs
        .into_iter()
        .map(|txout| {
            (
                *indices
                    .get(&(txout.script_pubkey.clone(), txout.value))
                    .unwrap_or(&usize::MAX), // Change amount
                txout,
            )
        })
        .collect();

    outputs_ordered.into_values().collect()
}
