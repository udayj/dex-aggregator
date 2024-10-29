use axum::{
    extract::Query,
    routing::{get, post},
    Json, Router,
};
use dex_aggregator::quotes::get_aggregator_quotes;
use dex_aggregator::types::Quote;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

// Define our API models

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

// API handlers
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
async fn get_quotes(Query(params): Query<Quote>) -> Json<Quote> {
    get_aggregator_quotes(params.clone()).await;
    Json(params)
}

#[tokio::main]
async fn main() {
    // Create API documentation
    let openapi = ApiDoc::openapi();

    // Build router with our endpoints and Swagger UI
    let app = Router::new()
        .route("/quotes", get(get_quotes))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi));

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://localhost:3000");
    println!("Swagger UI available at http://localhost:3000/swagger-ui/");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    //tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();

    /*axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .unwrap();*/
}
