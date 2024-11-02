use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use dex_aggregator::orchestrator::{
    get_aggregator_quotes, index_and_save_pair_data, index_and_save_path_data,
    index_and_save_pool_data, validate_request,
};
use dex_aggregator::types::{DexConfig, QuoteRequest, QuoteResponse, ResponsePool, Route};

use serde_json::json;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// Hold configuration
#[derive(Clone)]
struct DexConfigState {
    config: Arc<DexConfig>,
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

// Generate the OpenAPI schema
#[derive(OpenApi)]
#[openapi(
    paths(
        get_quotes,
        index_pair_data,
        index_path_data,
        index_pool_data
    ),
    components(
        schemas(QuoteRequest),
        schemas(QuoteResponse),
        schemas(Route),
        schemas(ResponsePool)
    ),
    tags(
        (name = "quotes", description = "Trade quotes for a token pair"),
    )
)]
struct ApiDoc;

#[utoipa::path(
get,
path = "/quotes",
params(
    ("sellTokenAddress" = String, Query, description = "Address of token being sold", example = "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8"),
    ("buyTokenAddress" = String, Query, description = "Address of token being bought", example = "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d"),
    ("sellAmount" = Option<String>, Query, description = "Amount of tokens being sold in decimal format (must be present if buyAmount not present) - if both present sellAmount is considered", example = "10000000000"),
    ("buyAmount" = Option<String>, Query, description = "Amount of tokens being bought in decimal format (must be present if sellAmount not present)", example = "210690000000000"),
    ("getLatest" = Option<bool>, Query, 
    description = "When true it indicates to server to get latest reserves from on-chain else use prior indexed data", 
    example = "true")
),
responses(
    (status = 200, description = "OK", body = QuoteResponse),
    (status = 400, description = "Bad Request"),
    (status = 500, description = "Internal Server Error")
),
tag = "quotes"
)]
async fn get_quotes(
    State(state): State<DexConfigState>,
    Query(params): Query<QuoteRequest>,
) -> Result<Json<QuoteResponse>, ApiError> {
    if let Err(e) = validate_request(state.config.as_ref(), &params) {
        return Err(ApiError::BadRequest(format!("{}", e)));
    }

    match get_aggregator_quotes(state.config.as_ref(), params.clone()).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(ApiError::Internal(format!("Failed to get quotes: {}", e))),
    }
}

#[utoipa::path(
    post,
    path = "/index_pair_data",
    responses(
        (status = 204, description = "Successfully updated pair data"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Index pair data"
)]
async fn index_pair_data(State(state): State<DexConfigState>) -> Result<StatusCode, ApiError> {
    index_and_save_pair_data(state.config.as_ref())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to update pair data: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/index_path_data",
    responses(
        (status = 204, description = "Successfully updated path data"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Index path data"
)]
async fn index_path_data(State(state): State<DexConfigState>) -> Result<StatusCode, ApiError> {
    index_and_save_path_data(state.config.as_ref())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to update path data: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/index_pool_data",
    responses(
        (status = 204, description = "Successfully updated pool data"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Index pool data"
)]
async fn index_pool_data(State(state): State<DexConfigState>) -> Result<StatusCode, ApiError> {
    index_and_save_pool_data(state.config.as_ref())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to update pool data: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

// API handlers
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create API documentation
    let openapi = ApiDoc::openapi();
    let config_path = PathBuf::from("dex_config.toml");
    let config_state = DexConfigState {
        config: Arc::new(DexConfig::load_from(config_path)?),
    };
    let app = Router::new()
        .route("/quotes", get(get_quotes))
        .route("/index_pair_data", post(index_pair_data))
        .route("/index_path_data", post(index_path_data))
        .route("/index_pool_data", post(index_pool_data))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))
        .with_state(config_state);

    println!("Server running on http://localhost:3000");
    println!("Swagger UI available at http://localhost:3000/swagger-ui/");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
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

// CONCERNS
// what happens if pair data is updated at the time that the pair data is being read by some other function

// Multi thread pair + pool
// config
// error handling
// references, copy trait
// post calls
// json output
// db abstraction
// generics
// read api key from env
