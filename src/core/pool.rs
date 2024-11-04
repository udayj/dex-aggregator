use super::constants::{GET_RESERVES_SELECTOR, SCALE};
use super::indexer::pool::{read_poolmap_data_from_disk, write_poolmap_data_on_disk};
use super::types::{Pool, PoolMap};
use super::Result;
use anyhow::Context;
use num_bigint::BigUint;
use num_traits::Zero;
use starknet::{
    core::types::{BlockId, Felt, FunctionCall},
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

fn create_pools_from_csv<P: AsRef<Path>>(path: P, required_tokens: &[String]) -> Result<PoolMap> {
    // Read pair data file and create empty pools for all supported tokens in the pool map
    let mut pool_map = PoolMap::new();
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();

        if parts.len() >= 3
            && required_tokens.contains(&parts[1])
            && required_tokens.contains(&parts[2])
        {
            let (token0, token1) = if BigUint::parse_bytes(parts[1].as_str()[2..].as_bytes(), 16)
                .unwrap()
                < BigUint::parse_bytes(parts[2].as_str()[2..].as_bytes(), 16).unwrap()
            {
                (parts[1].clone(), parts[2].clone())
            } else {
                (parts[2].clone(), parts[1].clone())
            };
            pool_map.insert(
                (token0, token1),
                Pool {
                    address: parts[0].clone(),
                    reserve0: BigUint::ZERO,
                    reserve1: BigUint::ZERO,
                    fee: BigUint::ZERO,
                    reserves_updated: false,
                    block_number: 0,
                },
            );
        }
    }

    Ok(pool_map)
}

// Function to get latest pool reserves data from rpc node and persist on disk through indexer
pub async fn index_latest_poolmap_data<P: AsRef<Path>>(
    rpc_url: &str,
    token_pair_file_path: P,
    poolmap_file_path: P,
    required_tokens: &[String],
) -> Result<()> {
    let (pool_map, _) = get_latest_pool_data(rpc_url, token_pair_file_path, required_tokens)
        .await
        .context("Error getting latest pool data while indexing poolmap data".to_string())?;

    write_poolmap_data_on_disk(poolmap_file_path, &pool_map)
        .context("Error writing poolmap data on disk".to_string())?;
    Ok(())
}

// Function to read previously indexed pool data from disk
// Faster than getting latest data from rpc node
pub fn get_indexed_pool_data<P: AsRef<Path>>(poolmap_file_path: P) -> Result<(PoolMap, u64)> {
    let pool_map = read_poolmap_data_from_disk(poolmap_file_path)?;
    let mut block_number = 0;
    if let Some((_, pool)) = pool_map.iter().next() {
        block_number = pool.block_number;
    }
    Ok((pool_map, block_number))
}

// Function to get upto date reserves data for all pools
// We can do this since we know beforehand that the number of pools can be at max 6C2 = 15
pub async fn get_latest_pool_data<P: AsRef<Path>>(
    rpc_url: &str,
    token_pair_file_path: P,
    required_tokens: &[String],
) -> Result<(PoolMap, u64)> {
    let pool_map = create_pools_from_csv(token_pair_file_path, required_tokens).unwrap();
    let pool_entries: Vec<((String, String), Pool)> = pool_map
        .iter()
        .map(|(pair, pool)| (pair.clone(), pool.clone()))
        .collect();
    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url).unwrap()));

    // Get latest accepted block number
    let block_number = provider.block_number().await?;

    // Initialize thread safe pool map from the empty pools created previously
    let arc_pool_map: Arc<Mutex<PoolMap>> = Arc::new(Mutex::new(pool_map.clone()));
    let mut threads = vec![];
    let rpc_url = rpc_url.to_string();
    for (pair, pool) in pool_entries {
        let shared_pool_map = arc_pool_map.clone();
        let rpc_url = rpc_url.clone();
        // Create separate threads to get latest reserves data asynchronously
        let worker_thread = tokio::spawn(async move {

            // Create sorted token pair
            let pool_key = if BigUint::parse_bytes(pair.0.as_str()[2..].as_bytes(), 16).unwrap()
                < BigUint::parse_bytes(pair.1.as_str()[2..].as_bytes(), 16).unwrap()
            {
                (pair.0.clone(), pair.1.clone())
            } else {
                (pair.1.clone(), pair.0.clone())
            };

            let provider =
                JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url.as_str()).unwrap()));
            let calldata = vec![];
            let mut byte_felts = vec![];

            let result = provider
                .call(
                    FunctionCall {
                        contract_address: Felt::from_hex(&pool.address).unwrap(),
                        entry_point_selector: Felt::from_hex(GET_RESERVES_SELECTOR).unwrap(),
                        calldata,
                    },
                    BlockId::Number(block_number),
                )
                .await
                .unwrap();

            for item in result.clone() {
                byte_felts.push(item.to_bytes_be());
            }
            let mut reserve0_bytes = vec![];
            reserve0_bytes.extend_from_slice(&byte_felts[1]);
            reserve0_bytes.extend_from_slice(&byte_felts[0]);
            let reserve0 = BigUint::from_bytes_be(&reserve0_bytes);

            let mut reserve1_bytes = vec![];
            reserve1_bytes.extend_from_slice(&byte_felts[3]);
            reserve1_bytes.extend_from_slice(&byte_felts[2]);
            let reserve1 = BigUint::from_bytes_be(&reserve1_bytes);

            let updated_pool = Pool {
                reserve0,
                reserve1,
                reserves_updated: true,
                address: pool.address.clone(),
                fee: pool.fee.clone(),
                block_number,
            };

            // Store updated pool reserves data in the shared pool map
            let mut shared_pool_map = shared_pool_map.lock().unwrap();
            shared_pool_map.insert(pool_key, updated_pool.clone());
        });

        threads.push(worker_thread);
    }

    for thread in threads.iter_mut() {
        thread.await.unwrap();
    }
    let output_pool_map = arc_pool_map.lock().unwrap().clone();
    Ok((output_pool_map, block_number))
}

impl Pool {
    pub fn get_amount_out(
        &self,
        amount_in: &BigUint,
        reserve0: &BigUint,
        reserve1: &BigUint,
    ) -> BigUint {
        // Constants for fee calculation
        // Fee could be a configurable value but using the constant here for simplicity and debuggability
        // since Jediswap has a constant DEX wise fee
        let fee_numerator = BigUint::from_str("3").unwrap(); // 0.3%
        let fee_denominator = BigUint::from_str("1000").unwrap(); // Base for percentage

        // Calculate amount_in after fee (amount_in * (1 - fee))
        let amount_in_with_fee = amount_in * (&fee_denominator - &fee_numerator);
        let numerator = &amount_in_with_fee * reserve1;
        let denominator = (reserve0 * &fee_denominator) + &amount_in_with_fee;

        if denominator.is_zero() {
            return BigUint::ZERO;
        }
        numerator / denominator
    }

    pub fn get_amount_in(
        &self,
        amount_out: &BigUint,
        reserve0: &BigUint,
        reserve1: &BigUint,
    ) -> Option<BigUint> {
        // Constants for fee calculation
        let fee_numerator = BigUint::from_str("3").unwrap(); // 0.3%
        let fee_denominator = BigUint::from_str("1000").unwrap(); // Base for percentage

        if amount_out >= reserve1 {
            return None;
        }
        let numerator = amount_out * reserve0 * &fee_denominator;
        let denominator = (reserve1 - amount_out) * (&fee_denominator - &fee_numerator);

        // Calculate final amount in and round up
        Some((&numerator + &denominator - BigUint::from(1u32)) / denominator)
    }

    pub fn to_f64(value: &BigUint) -> f64 {
        let value_str = value.to_string();
        //let scaling_str = scaling_factor.to_string();

        let value_f64 = value_str.parse::<f64>().unwrap_or(0.0);
        // let scaling_f64 = scaling_str.parse::<f64>().unwrap_or(1.0);

        value_f64 / SCALE
    }

    // Convert f64 to BigUint, properly handling the scaling factor
    pub fn from_f64(value: f64) -> BigUint {
        if value <= 0.0 {
            return BigUint::zero();
        }

        // Convert the float to a scaled integer to maintain precision
        let scaled_value = (value * SCALE) as u64;
        BigUint::from(scaled_value)
    }
}
