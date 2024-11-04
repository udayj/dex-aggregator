use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type TokenPath = Vec<String>;
pub type PathMap = HashMap<(String, String), Vec<TokenPath>>;
pub type PoolMap = HashMap<(String, String), Pool>;

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
pub struct PathKey(String, String);

#[derive(Debug)]
pub struct Graph {
    pub edges: HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pool {
    pub address: String,
    pub reserve0: BigUint,
    pub reserve1: BigUint,
    pub fee: BigUint, // unused since fee is constant
    pub reserves_updated: bool,
    pub block_number: u64,
}

#[derive(Clone, Debug)]
pub struct TradePath {
    pub tokens: Vec<String>, // ["A", "B", "C"] for path A->B->C
}

// Holds state for an optimizer
#[derive(Clone)]
pub struct Optimizer {
    pub paths: Vec<TradePath>,
    pub pools: PoolMap,
    pub total_amount: BigUint,
}
