use stacks_signer_api::{
    db,
    routes::{
        keys::{add_key_route, delete_key_route, get_keys_route},
        signers::{add_signer_route, delete_signer_route, get_signers_route},
    },
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use warp::Filter;

pub fn initiate_tracing_subscriber() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

#[tokio::main]
async fn main() {
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
    let get_signers_route = get_signers_route(pool);

    let routes = add_key_route
        .or(delete_key_route)
        .or(get_key_route)
        .or(add_signer_route)
        .or(delete_signer_route)
        .or(get_signers_route);

    // Run the server
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
