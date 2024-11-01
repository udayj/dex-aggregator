use super::constants::GET_RESERVES_SELECTOR;
use super::types::{Pool, PoolMap, TradePath};
use num_bigint::BigUint;
use num_traits::{CheckedSub, ConstZero, One, Zero};
use serde::{Deserialize, Serialize};
use starknet::{
    core::types::{
        BlockId, BlockTag, EventFilter, Felt, FunctionCall, MaybePendingBlockWithTxHashes,
    },
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::error::Error;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::ops::Mul;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, fs::File};

const SCALE: f64 = 1000000 as f64;

fn create_pools_from_csv<P: AsRef<Path>>(
    path: P,
    required_tokens: &[String],
) -> io::Result<PoolMap> {
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
            let (token0, token1) = if BigUint::parse_bytes(&parts[1].as_str()[2..].as_bytes(), 16)
                .unwrap()
                < BigUint::parse_bytes(&parts[2].as_str()[2..].as_bytes(), 16).unwrap()
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

pub async fn index_latest_poolmap_data<P: AsRef<Path>>(
    rpc_url: &str,
    token_pair_file_path: P,
    poolmap_file_path: P,
    required_tokens: &[String],
) -> Result<(), Box<dyn Error>> {
    let pool_map = get_latest_pool_data(rpc_url, token_pair_file_path, required_tokens).await?;

    let pool_list = PoolList::from_hash_map(&pool_map);
    let json = serde_json::to_string_pretty(&pool_list)?;

    fs::write(poolmap_file_path, json)?;
    Ok(())
}

pub async fn get_indexed_pool_data<P: AsRef<Path>>(
    poolmap_file_path: P,
) -> Result<PoolMap, Box<dyn Error>> {
    let pool_list_json = fs::read_to_string(poolmap_file_path)?;
    let pool_list: PoolList = serde_json::from_str(&pool_list_json)?;
    let pool_map = pool_list.to_hash_map();
    Ok(pool_map)
}

pub async fn get_latest_pool_data<P: AsRef<Path>>(
    rpc_url: &str,
    token_pair_file_path: P,
    required_tokens: &[String],
) -> Result<PoolMap, Box<dyn Error>> {
    let mut pool_map = create_pools_from_csv(token_pair_file_path, required_tokens).unwrap();
    let pool_entries: Vec<((String, String), Pool)> = pool_map
        .iter()
        .map(|(pair, pool)| (pair.clone(), pool.clone()))
        .collect();
    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url).unwrap()));
    let block_number = provider.block_number().await?;
    let arc_pool_map: Arc<Mutex<PoolMap>> = Arc::new(Mutex::new(pool_map.clone()));
    let mut threads = vec![];
    let rpc_url = rpc_url.to_string();
    for (pair, pool) in pool_entries {
        let mut shared_pool_map = arc_pool_map.clone();
        let rpc_url = rpc_url.clone();
        let worker_thread = tokio::spawn(async move {
            let pool_key = if BigUint::parse_bytes(&pair.0.as_str()[2..].as_bytes(), 16).unwrap()
                < BigUint::parse_bytes(pair.1.as_str()[2..].as_bytes(), 16).unwrap()
            {
                (pair.0.clone(), pair.1.clone())
            } else {
                (pair.1.clone(), pair.0.clone())
            };

            let provider =
                JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url.as_str()).unwrap()));
            let mut calldata = vec![];
            //let mut str_felts = vec![];
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
            let mut shared_pool_map = shared_pool_map.lock().unwrap();
            shared_pool_map.insert(pool_key, updated_pool.clone());
        });

        threads.push(worker_thread);
    }

    for thread in threads.iter_mut() {
        thread.await.unwrap();
    }
    let output_pool_map = arc_pool_map.lock().unwrap().clone();
    Ok(output_pool_map)
}

impl Pool {
    pub fn get_amount_out(
        &self,
        amount_in: &BigUint,
        reserve0: &BigUint,
        reserve1: &BigUint,
    ) -> BigUint {
        // Constants for fee calculation
        let fee_numerator = BigUint::from_str("3").unwrap(); // 0.3%
        let fee_denominator = BigUint::from_str("1000").unwrap(); // Base for percentage

        // Calculate amount_in after fee (amount_in * (1 - fee))
        let amount_in_with_fee = amount_in * (&fee_denominator - &fee_numerator);

        // Calculate numerator: amount_in_with_fee * reserve1
        let numerator = &amount_in_with_fee * reserve1;

        // Calculate denominator: (reserve0 * fee_denominator) + amount_in_with_fee
        let denominator = (reserve0 * &fee_denominator) + &amount_in_with_fee;

        // Protect against division by zero
        if denominator.is_zero() {
            return BigUint::ZERO;
        }

        // Calculate final amount out
        numerator / denominator
    }

    pub fn get_amount_in(
        &self,
        amount_out: &BigUint,
        reserve0: &BigUint,
        reserve1: &BigUint,
    ) -> BigUint {
        // Constants for fee calculation
        let fee_numerator = BigUint::from_str("3").unwrap(); // 0.3%
        let fee_denominator = BigUint::from_str("1000").unwrap(); // Base for percentage

        // Check if amount_out is greater than reserve1
        if amount_out >= reserve1 {
            return BigUint::from_str("1000000000000000000000000000").unwrap();
        }

        // Calculate numerator: amount_out * reserve0 * fee_denominator
        let numerator = amount_out * reserve0 * &fee_denominator;

        // Calculate denominator: (reserve1 - amount_out) * (fee_denominator - fee_numerator)
        let denominator = (reserve1 - amount_out) * (&fee_denominator - &fee_numerator);

        // Protect against division by zero
        //if denominator.is_zero() {
        //    return None;
        //}

        // Calculate final amount in and round up
        let amount_in = (&numerator + &denominator - BigUint::from(1u32)) / denominator;

        amount_in
    }

    pub fn to_f64(value: &BigUint) -> f64 {
        let value_scaled = value.mul(BigUint::from(1_000_000u32)); // Additional scaling for precision
                                                                   //let scaling_factor_scaled = scaling_factor.mul(BigUint::from(1_000_000u32));

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
        let base = BigUint::from(scaled_value);

        // Scale back down and multiply by scaling factor
        return base;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PoolEntry {
    token0: String,
    token1: String,
    pool: Pool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PoolList {
    pools: Vec<PoolEntry>,
}

impl PoolList {
    fn from_hash_map(map: &HashMap<(String, String), Pool>) -> Self {
        let pools = map
            .iter()
            .map(|((token0, token1), pool)| PoolEntry {
                token0: token0.clone(),
                token1: token1.clone(),
                pool: pool.clone(),
            })
            .collect();

        Self { pools }
    }

    fn to_hash_map(&self) -> HashMap<(String, String), Pool> {
        self.pools
            .iter()
            .map(|entry| {
                (
                    (entry.token0.clone(), entry.token1.clone()),
                    entry.pool.clone(),
                )
            })
            .collect()
    }
}
