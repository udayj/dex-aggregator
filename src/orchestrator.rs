use crate::core::constants::INFINITE;

use super::core::optimization::{optimize_amount_in, optimize_amount_out};
use super::core::pair::index_latest_pair_data;
use super::core::path::{get_paths_between, update_path_data, update_pathmap};
use super::core::pool::{get_indexed_pool_data, get_latest_pool_data, index_latest_poolmap_data};
use super::core::types::{Pool, TradePath};
use super::types::{DexConfig, QuoteRequest, QuoteResponse, ResponsePool, Route};
use anyhow::{anyhow, Result};
use num_bigint::BigUint;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// Orchestrator is a middleware between the core dex components and the main REST service entry points

// Function to validate requests
pub fn validate_request(config: &DexConfig, request: &QuoteRequest) -> Result<()> {
    if request.buyTokenAddress.trim().is_empty() || request.sellTokenAddress.trim().is_empty() {
        return Err(anyhow!("Buy and Sell Token addresses cannot be empty"));
    }

    if !config.supported_tokens.contains(&request.buyTokenAddress)
        || !config.supported_tokens.contains(&request.sellTokenAddress)
    {
        return Err(anyhow!("Unsupported token address"));
    }

    if request.buyAmount.is_none() && request.sellAmount.is_none() {
        return Err(anyhow!("Sell Amount is mandatory"));
    }

    let num;
    if request.sellAmount.is_some() {
        num = request.sellAmount.clone().unwrap().parse::<u128>().map_err(
            |_| anyhow!("Invalid amount")
        )?;
        
    }
    else {
        num  = request.buyAmount.clone().unwrap().parse::<u128>().map_err(
            |_| anyhow!("Invalid amount")
        )?;
    }
    if num == 0 {
        return Err(anyhow!("Amount cannot be 0"));
    }
    Ok(())
}

// Create working directory and get fresh pair data from rpc node
pub async fn index_and_save_pair_data(config: &DexConfig) -> Result<()> {
    if !Path::new(config.working_dir.as_str()).exists() {
        fs::create_dir(config.working_dir.clone())?;
    }

    let dir = Path::new(config.working_dir.as_str());
    let pair_file_path = dir.join(config.pair_file.clone());
    let dir = Path::new(config.working_dir.as_str());
    let token_pair_file_path = dir.join(config.token_pair_file.clone());

    index_latest_pair_data(
        &config.rpc_url,
        pair_file_path.to_str().unwrap(),
        token_pair_file_path.to_str().unwrap(),
    )
    .await?;
    Ok(())
}

// Recalculate and index fresh path data on disk
// Required to be called only if pair data has changed or supported token list has changed
pub async fn index_and_save_path_data(config: &DexConfig) -> Result<()> {
    if !Path::new(config.working_dir.as_str()).exists() {
        return Err(anyhow!("Token Pair data file not found"));
    }
    let dir = Path::new(config.working_dir.as_str());
    let token_pair_file_path = dir.join(config.token_pair_file.clone());

    let dir = Path::new(config.working_dir.as_str());
    let pathmap_file_path = dir.join(config.pathmap_file.clone());

    let mut output_paths = vec![];
    for token in &config.supported_tokens {
        let dir = Path::new(config.working_dir.as_str());
        let token_paths_file = dir.join(token.clone() + ".txt");
        output_paths.push(token_paths_file);
    }
    update_path_data(
        token_pair_file_path,
        &config.supported_tokens,
        &output_paths,
    )?;
    update_pathmap(pathmap_file_path, &output_paths)?;
    Ok(())
}

pub async fn index_and_save_pool_data(config: &DexConfig) -> Result<()> {
    if !Path::new(config.working_dir.as_str()).exists() {
        return Err(anyhow!("Token Pair data file not found"));
    }

    let dir = Path::new(config.working_dir.as_str());
    let token_pair_file_path = dir.join(config.token_pair_file.clone());

    let dir = Path::new(config.working_dir.as_str());
    let poolmap_file_path = dir.join(config.poolmap_file.clone());
    index_latest_poolmap_data(
        &config.rpc_url,
        token_pair_file_path,
        poolmap_file_path,
        &config.supported_tokens,
    )
    .await?;
    Ok(())
}

// Function to get trade quotes from the core dex
pub async fn get_aggregator_quotes(
    config: &DexConfig,
    params: QuoteRequest,
) -> Result<QuoteResponse> {
    if !Path::new(config.working_dir.as_str()).exists() {
        return Err(anyhow!("Token Pair/Path data files not found"));
    }
    let dir = Path::new(config.working_dir.as_str());
    let pathmap_file_path = dir.join(config.pathmap_file.clone());

    let dir = Path::new(config.working_dir.as_str());
    let token_pair_file_path = dir.join(config.token_pair_file.clone());

    // Get all possible paths between token pair
    let required_trade_paths = get_paths_between(
        pathmap_file_path,
        params.sellTokenAddress.clone(),
        params.buyTokenAddress.clone(),
    )
    .map_err(|e| anyhow!(format!("{}", e)))?;

    // Get latest/indexed pool reserves data based on query parameter getLatest
    let (pool_map, block_number) = if params.getLatest.is_some_and(|x| x) {
        get_latest_pool_data(
            &config.rpc_url,
            token_pair_file_path,
            &config.supported_tokens,
        )
        .await
        .map_err(|e| anyhow!(format!("{}", e)))?
    } else {
        let dir = Path::new(config.working_dir.as_str());
        let poolmap_file_path = dir.join(config.poolmap_file.clone());
        get_indexed_pool_data(poolmap_file_path).map_err(|e| anyhow!(format!("{}", e)))?
    };

    if params.sellAmount.is_some() {
        // When selling amount is given, we do not consider buyAmount
        let (splits, total_amount) = optimize_amount_out(
            required_trade_paths.clone(),
            pool_map.clone(),
            BigUint::from(params.sellAmount.clone().unwrap().parse::<u128>().unwrap()),
        );

        // Form output structure that can be converted to json format as a response from the REST api service
        let routes: Vec<Route> = splits
            .iter()
            .zip(required_trade_paths.iter())
            .filter_map(|(split, trade_path)| {
                if *split != BigUint::ZERO {
                    Some(Route {
                        percent: Pool::to_f64(split),
                        path: build_response_path(trade_path, &pool_map),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(QuoteResponse {
            sellTokenAddress: params.sellTokenAddress.clone(),
            buyTokenAddress: params.buyTokenAddress.clone(),
            sellAmount: params.sellAmount.unwrap(),
            buyAmount: total_amount.to_string(),
            blockNumber: block_number,
            chainId: config.chain_id.clone(),
            routes,
        })
    } else {
        let (splits, total_amount) = optimize_amount_in(
            required_trade_paths.clone(),
            pool_map.clone(),
            BigUint::from(params.buyAmount.clone().unwrap().parse::<u128>().unwrap()),
        );
        
        // Form output structure that can be converted to json format as a response from the REST api service
        let routes: Vec<Route> = splits
            .iter()
            .zip(required_trade_paths.iter())
            .filter_map(|(split, trade_path)| {
                if *split != BigUint::ZERO {
                    Some(Route {
                        percent: Pool::to_f64(split),
                        path: build_response_path(trade_path, &pool_map),
                    })
                } else {
                    None
                }
            })
            .collect();
        let total_amount = if total_amount == INFINITE() {
            "INFINITE".to_string()
        } else {
            total_amount.to_string()
        };
        Ok(QuoteResponse {
            sellTokenAddress: params.sellTokenAddress.clone(),
            buyTokenAddress: params.buyTokenAddress,
            sellAmount: total_amount,
            buyAmount: params.buyAmount.unwrap(),
            blockNumber: block_number,
            chainId: config.chain_id.clone(),
            routes,
        })
    }
}


// Form response path comprising ResponsePools for a TradePath
fn build_response_path(
    trade_path: &TradePath,
    pool_map: &HashMap<(String, String), Pool>,
) -> Vec<ResponsePool> {
    trade_path
        .tokens
        .windows(2)
        .map(|window| build_response_pool(&window[0], &window[1], pool_map).unwrap())
        .collect()
}

fn build_response_pool(
    token_in: &str,
    token_out: &str,
    pool_map: &HashMap<(String, String), Pool>,
) -> Option<ResponsePool> {
    // TODO - Consider moving this to a configurable section
    let symbol_list: Vec<(&str, &str)> = vec![
        (
            "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            "ETH",
        ),
        (
            "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
            "USDT",
        ),
        (
            "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
            "USDC",
        ),
        (
            "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
            "wstETH",
        ),
        (
            "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
            "DAI",
        ),
        (
            "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
            "STRK",
        ),
    ];

    let symbol_map: HashMap<String, String> = symbol_list
        .iter()
        .map(|token| (token.0.to_string(), token.1.to_string()))
        .collect();
    // Try to find pool in both directions
    let pool = pool_map
        .get(&(token_in.to_string(), token_out.to_string()))
        .or_else(|| pool_map.get(&(token_out.to_string(), token_in.to_string())));

    pool.map(|p| ResponsePool {
        pairAddress: p.address.clone(),
        tokenIn: token_in.to_string(),
        tokenOut: token_out.to_string(),
        tokenInSymbol: symbol_map.get(token_in).unwrap().to_string(), 
        tokenOutsymbol: symbol_map.get(token_out).unwrap().to_string(), 
    })
}
