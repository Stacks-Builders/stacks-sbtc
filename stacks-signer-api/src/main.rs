use std::net::SocketAddr;

use blockstack_lib::burnchains::Txid;
use clap::Parser;
use rand::Rng;
use stacks_signer_api::{
    db::{self, transaction::add_transaction, vote::add_vote},
    routes::{
        keys::{add_key_route, delete_key_route, get_keys_route},
        signers::{add_signer_route, delete_signer_route, get_signers_route},
        transaction::{get_transaction_by_id_route, get_transactions_route},
        vote::vote_route,
    },
    transaction::{Transaction, TransactionAddress, TransactionKind, TransactionResponse},
    vote::{Vote, VoteChoice, VoteMechanism, VoteRequest, VoteResponse, VoteStatus, VoteTally},
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use utoipa::OpenApi;
use warp::Filter;

#[derive(OpenApi)]
#[openapi(
    paths(stacks_signer_api::routes::transaction::get_transaction_by_id),
    components(
        schemas(
            Transaction,
            VoteChoice,
            VoteMechanism,
            VoteRequest,
            VoteStatus,
            VoteTally
        ),
        responses(TransactionResponse, VoteResponse)
    )
)]
struct ApiDoc;

pub fn initiate_tracing_subscriber() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Port to run API server on
    #[arg(short, long, default_value = "3030")]
    pub port: u16,
    /// Address to run API server on
    #[arg(short, long, default_value = "127.0.0.1")]
    pub address: String,
    /// Insert dummy values in db
    #[arg(short, long, default_value = "false")]
    pub dummy: bool,
    /// Database file path
    #[arg(long)]
    pub db: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    println!("{}", ApiDoc::openapi().to_pretty_json().unwrap());
    // First enable tracing
    initiate_tracing_subscriber();

    // Initialize the connection pool
    let pool = db::init_pool(cli.db)
        .await
        .expect("Failed to initialize pool.");
    if cli.dummy {
        let (txs, votes) = generate_dummy_txs_votes();
        for tx in txs {
            add_transaction(&pool, &tx)
                .await
                .expect("Failed to add transaction.");
        }
        for vote in votes {
            add_vote(&vote, &pool).await.expect("Failed to add vote.");
        }
    }
    // Set up the routes
    // Key routes
    let add_key_route = add_key_route(pool.clone());
    let delete_key_route = delete_key_route(pool.clone());
    let get_key_route = get_keys_route(pool.clone());
    // Signer routes
    let add_signer_route = add_signer_route(pool.clone());
    let delete_signer_route = delete_signer_route(pool.clone());
    let get_signers_route = get_signers_route(pool.clone());
    // Transaction routes
    let get_transactions_route = get_transactions_route(pool.clone());
    let get_transaction_by_id_route = get_transaction_by_id_route(pool.clone());
    // Vote routes
    let vote_route = vote_route(pool);

    // Combine the routes
    let routes = add_key_route
        .or(delete_key_route)
        .or(get_key_route)
        .or(add_signer_route)
        .or(delete_signer_route)
        .or(get_signers_route)
        .or(get_transactions_route)
        .or(get_transaction_by_id_route)
        .or(vote_route);

    // Run the server
    let server = format!("{}:{}", cli.address, cli.port);
    let server: SocketAddr = server
        .parse()
        .expect("Failed to parse provided address and port into socket address");
    warp::serve(routes).run(server).await;
}

/// Generate some dummy transactions for mocked backend
fn generate_dummy_txs_votes() -> (Vec<Transaction>, Vec<Vote>) {
    let mut txs = vec![];
    let mut votes = vec![];
    for i in 0..10 {
        let tx = generate_dummy_transaction(i);
        votes.push(generate_dummy_vote(&tx));
        txs.push(tx);
    }
    (txs, votes)
}

fn generate_dummy_vote(txs: &Transaction) -> Vote {
    let mut rng = rand::thread_rng();
    let txid = txs.txid;
    let vote_mechanism = if rng.gen_range(0..2) == 0 {
        VoteMechanism::Auto
    } else {
        VoteMechanism::Manual
    };
    let status = rng.gen_range(0..4);
    let (vote_status, current_consensus) = if status == 0 {
        (VoteStatus::Pending, rng.gen_range(1..55))
    } else if status == 1 {
        (VoteStatus::Approved, rng.gen_range(70..100))
    } else if status == 2 {
        (VoteStatus::Rejected, rng.gen_range(70..100))
    } else {
        (VoteStatus::NoConsensus, rng.gen_range(1..69))
    };
    let choice = rng.gen_range(0..3);
    let vote_choice = if vote_mechanism == VoteMechanism::Auto {
        if choice == 0 {
            Some(VoteChoice::Approve)
        } else {
            Some(VoteChoice::Reject)
        }
    } else if choice == 0 {
        Some(VoteChoice::Approve)
    } else if choice == 1 {
        Some(VoteChoice::Reject)
    } else {
        None
    };
    Vote {
        txid,
        vote_mechanism,
        vote_tally: VoteTally {
            current_consensus,
            target_consensus: 70,
            vote_status,
        },
        vote_choice,
    }
}

fn generate_dummy_transaction(i: usize) -> Transaction {
    let mut rng = rand::thread_rng();
    let rand_kind = rng.gen_range(0..4);
    let transaction_kind = if rand_kind == 0 {
        TransactionKind::DepositReveal
    } else if rand_kind == 1 {
        TransactionKind::WithdrawalReveal
    } else if rand_kind == 2 {
        TransactionKind::WithdrawalFulfill
    } else {
        TransactionKind::WalletHandoff
    };
    let transaction_block_height = rng.gen();
    Transaction {
        txid: Txid::from([i as u8; 32]),
        transaction_kind,
        transaction_block_height,
        transaction_deadline_block_height: transaction_block_height.unwrap_or(0)
            + rng.gen_range(1..10),
        transaction_amount: rng.gen(),
        transaction_fees: rng.gen_range(10..1000),
        memo: vec![],
        transaction_originator_address: TransactionAddress::Bitcoin("originator".to_string()),
        transaction_debit_address: TransactionAddress::Bitcoin("escrow bitcoin wallet".to_string()),
        transaction_credit_address: TransactionAddress::Stacks("sBTC protocol address".to_string()),
    }
}
