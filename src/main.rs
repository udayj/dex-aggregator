use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use dex_aggregator::quotes::get_aggregator_quotes;
use dex_aggregator::types::{Quote, DexConfig};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use dex_aggregator::config;
use std::error::Error;
use std::sync::Arc;
use std::path::PathBuf;

// Hold configuration
#[derive(Clone)]
struct DexConfigState {
    config: Arc<DexConfig>,
}


// Generate the OpenAPI schema
#[derive(OpenApi)]
#[openapi(
    paths(
        get_quotes,
    ),
    components(
        schemas(Quote)
    ),
    tags(
        (name = "quotes", description = "Trade quotes for a token pair")
    )
)]
struct ApiDoc;

#[utoipa::path(
get,
path = "/quotes",
params(
    ("sellTokenAddress" = String, Query, description = "Address of token being sold"),
    ("buyTokenAddress" = String, Query, description = "Address of token being bought"),
    ("sellAmount" = Option<String>, Query, description = "Amount of tokens being sold"),
    ("buyAmount" = Option<String>, Query, description = "Amount of tokens being bought")
),
responses(
    (status = 200, description = "Trade Quote", body = Quote)
),
tag = "quotes"
)]
async fn get_quotes(State(state): State<DexConfigState>, Query(params): Query<Quote>) -> Json<Quote> {
    get_aggregator_quotes(state.config.as_ref(), params.clone()).await;
    Json(params)
}

// API handlers


#[tokio::main]
async fn main() -> Result<(),Box<dyn Error>> {
    // Create API documentation
    let openapi = ApiDoc::openapi();
    let config_path = PathBuf::from("dex_config.toml");

    let config_state = DexConfigState {
        config: Arc::new(DexConfig::load_from(config_path)?)
    };
    // Build router with our endpoints and Swagger UI
    let app = Router::new()
        .route("/quotes", get(get_quotes))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))
        .with_state(config_state);

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://localhost:3000");
    println!("Swagger UI available at http://localhost:3000/swagger-ui/");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await?;
    //tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;

    /*axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .unwrap();*/
    Ok(())
}

// input json
// token in
// token out
// sell amount - amount in
// buy amount - amount out
// get_latest - bool

// output json
// token in
// token out
// sell amount - amount in
// buy amount - amount out
// block number - "latest"/block_number based on pool data
// chain id
// routes [(percent:, (pair address, token in, token out, token in symbol, token out symbol)]


// post endpoints - update pair data, store paths on disk & path map on disk, update latest pool data for all pools
// all the above should persist data in storage, and other read functions should read data from storage
// get_paths_between should read pathmap from storage
// get_pooldata - should simply read from storage and return
// pair and path data should be updated together since path data changes only if pair data changes
// pool data can be updated independently
// all data files in separate working directory


// what happens if pair data is updated at the time that the pair data is being read by some other function
