use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};


pub type TokenPath = Vec<String>;
pub type PathMap = HashMap<(String, String), Vec<TokenPath>>;
pub type PoolMap = HashMap<(String, String), Pool>;


#[derive(Serialize, Deserialize, ToSchema, IntoParams, Clone)]
pub struct Quote {
    #[schema(example = "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8")]
    pub sellTokenAddress: String,

    #[schema(example = "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d")]
    pub buyTokenAddress: String,

    #[schema(example = "1000000", nullable = true)]
    pub sellAmount: Option<String>,

    #[schema(example = "2106900000", nullable = true)]
    pub buyAmount: Option<String>,

    #[schema(example = "true", default = false)]
    pub getLatest: bool
}

#[derive(Debug)]
pub struct Graph {
    pub edges: HashMap<String, Vec<String>>,
}


#[derive(Clone, Debug)]
pub struct Pool {
    pub address: String,
    pub reserve0: BigUint,
    pub reserve1: BigUint,
    pub fee: BigUint, // Fee in basis points
    pub reserves_updated: bool,
}

#[derive(Clone, Debug)]
pub struct TradePath {
    pub tokens: Vec<String>, // ["A", "B", "C"] for path A->B->C
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
}