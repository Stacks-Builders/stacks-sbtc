use std::{net::SocketAddr, sync::Arc};

use blockstack_lib::burnchains::Txid;
use clap::Parser;
use rand::Rng;
use sqlx::SqlitePool;
use stacks_signer_api::{
    db::{self, transaction::add_transaction, vote::add_vote},
    routes::all_routes,
    transaction::{Transaction, TransactionAddress, TransactionKind, TransactionResponse},
    vote::{Vote, VoteChoice, VoteMechanism, VoteRequest, VoteResponse, VoteStatus, VoteTally},
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use utoipa::OpenApi;
use utoipa_swagger_ui::Config;
use warp::{
    http::Uri,
    hyper::{Response, StatusCode},
    path::{FullPath, Tail},
    Filter, Rejection, Reply,
};

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

/// The available CLI subcommands
#[derive(clap::Subcommand, Debug, Clone)]
enum Command {
    Docs(DocsArgs),
    Swagger(SwaggerArgs),
    Dummy(DummyArgs),
    Run(RunArgs),
}

#[derive(Parser, Debug, Clone)]
struct ServerArgs {
    /// Port to run API server on
    #[arg(short, long, default_value = "3030")]
    pub port: u16,
    /// Address to run API server on
    #[arg(short, long, default_value = "0.0.0.0")]
    pub address: String,
}

#[derive(Parser, Debug, Clone)]
struct DocsArgs {
    //Output file to save docs to. Prints to stdout if not provided
    #[arg(long, short)]
    pub output: Option<String>,
}

#[derive(Parser, Debug, Clone)]
struct SwaggerArgs {
    /// Port and Address to run Swagger UI server on
    #[command(flatten)]
    pub server: ServerArgs,
}

#[derive(Parser, Debug, Clone)]
struct DummyArgs {
    /// Port and address to run API server on
    #[command(flatten)]
    pub server: ServerArgs,
}

#[derive(Parser, Debug, Clone)]
struct RunArgs {
    /// Port and address to run API server on
    #[command(flatten)]
    pub server: ServerArgs,
    /// Database file path
    #[arg(long)]
    pub db: Option<String>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Subcommand action to take
    #[command(subcommand)]
    command: Command,
}

async fn run(pool: SqlitePool, server_args: ServerArgs) {
    // Create the routes
    let routes = all_routes(pool);

    // Run the warp server
    let server = format!("{}:{}", server_args.address, server_args.port);
    let server: SocketAddr = server
        .parse()
        .expect("Failed to parse provided address and port into socket address");
    println!("Serving warp server on {}", server);
    warp::serve(routes).run(server).await;
}

async fn run_swagger(pool: SqlitePool, args: SwaggerArgs) {
    let config = Arc::new(Config::from("/api-doc.json"));
    let api_doc = warp::path("api-doc.json")
        .and(warp::get())
        .map(|| warp::reply::json(&ApiDoc::openapi()));

    let swagger_ui = warp::path("swagger-ui")
        .and(warp::get())
        .and(warp::path::full())
        .and(warp::path::tail())
        .and(warp::any().map(move || config.clone()))
        .and_then(serve_swagger);

    let server = format!("{}:{}", args.server.address, args.server.port);
    let server: SocketAddr = server
        .parse()
        .expect("Failed to parse provided address and port into socket address");
    println!(
        "Serving swagger UI on http://{}:{}/swagger-ui/",
        args.server.address, args.server.port
    );

    warp::serve(api_doc.or(swagger_ui).or(all_routes(pool)))
        .run(server)
        .await;
}

async fn serve_swagger(
    full_path: FullPath,
    tail: Tail,
    config: Arc<Config<'static>>,
) -> Result<Box<dyn Reply + 'static>, Rejection> {
    if full_path.as_str() == "/swagger-ui" {
        return Ok(Box::new(warp::redirect::found(Uri::from_static(
            "/swagger-ui/",
        ))));
    }

    let path = tail.as_str();
    match utoipa_swagger_ui::serve(path, config) {
        Ok(file) => {
            if let Some(file) = file {
                Ok(Box::new(
                    Response::builder()
                        .header("Content-Type", file.content_type)
                        .body(file.bytes),
                ))
            } else {
                Ok(Box::new(StatusCode::NOT_FOUND))
            }
        }
        Err(error) => Ok(Box::new(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(error.to_string()),
        )),
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // First enable tracing
    initiate_tracing_subscriber();

    // Initialize the connection pool
    let pool = db::init_pool(None)
        .await
        .expect("Failed to initialize pool.");

    match cli.command {
        Command::Docs(args) => {
            let docs = ApiDoc::openapi();
            let openapi_json = docs
                .to_pretty_json()
                .expect("Failed to generate OpenAPI json docs.");
            if let Some(output_file) = args.output {
                std::fs::write(output_file, openapi_json)
                    .expect("Failed to write OpenAPI json docs to file.");
                return;
            }
            println!("{}", openapi_json);
        }
        Command::Swagger(args) => {
            run_swagger(pool, args).await;
        }
        Command::Dummy(args) => {
            let (txs, votes) = generate_dummy_txs_votes();
            for tx in txs {
                add_transaction(&pool, &tx)
                    .await
                    .expect("Failed to add transaction.");
            }
            for vote in votes {
                add_vote(&vote, &pool).await.expect("Failed to add vote.");
            }

            run(pool, args.server).await;
        }
        Command::Run(args) => {
            run(pool, args.server).await;
        }
    }
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
