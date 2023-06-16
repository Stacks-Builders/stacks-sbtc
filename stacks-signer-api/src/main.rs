use std::net::SocketAddr;

use clap::Parser;
use stacks_signer_api::{
    db,
    routes::{
        keys::{add_key_route, delete_key_route, get_keys_route},
        signers::{add_signer_route, delete_signer_route, get_signers_route},
        transaction::{get_transaction_by_id_route, get_transactions_route}, vote::vote_route,
    },
    transaction::{Transaction, TransactionResponse},
    vote::{VoteChoice, VoteMechanism, VoteRequest, VoteResponse, VoteStatus, VoteTally},
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use warp::Filter;

use utoipa::OpenApi;

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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    println!("{}", ApiDoc::openapi().to_pretty_json().unwrap());
    // First enable tracing
    initiate_tracing_subscriber();

    // Initialize the connection pool
    let pool = db::init_pool(None)
        .await
        .expect("Failed to initialize pool.");

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
    let get_transactions_route = get_transactions_route();
    let get_transaction_by_id_route = get_transaction_by_id_route();
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
    let server: SocketAddr = server.parse().expect(&format!(
        "Failed to parse provided address and port into socket address: {}",
        server
    ));
    warp::serve(routes).run(server).await;
}
