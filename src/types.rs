use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, ToSchema, IntoParams, Clone)]
pub struct QuoteRequest {
    #[schema(example = "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8")]
    pub sellTokenAddress: String,

    #[schema(example = "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d")]
    pub buyTokenAddress: String,

    #[schema(example = "1000000")]
    pub sellAmount: Option<String>,

    #[schema(example = "2106900000")]
    pub buyAmount: Option<String>,

    #[schema(example = "true", default = false, nullable = true)]
    pub getLatest: Option<bool>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ResponsePool {
    pub pairAddress: String,
    pub tokenIn: String,
    pub tokenOut: String,
    pub tokenInSymbol: String,
    pub tokenOutsymbol: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Route {
    pub percent: f64,
    pub path: Vec<ResponsePool>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, ToSchema)]
pub struct QuoteResponse {
    pub sellTokenAddress: String,
    pub buyTokenAddress: String,
    pub sellAmount: String,
    pub buyAmount: String,
    pub blockNumber: u64,
    pub chainId: String,
    pub routes: Vec<Route>,
}

#[derive(Serialize, Deserialize)]
pub struct DexConfig {
    pub working_dir: String,
    pub pair_file: String,
    pub token_pair_file: String,
    pub supported_tokens: Vec<String>,
    pub pathmap_file: String,
    pub poolmap_file: String,
    pub rpc_url: String,
    pub chain_id: String,
}
