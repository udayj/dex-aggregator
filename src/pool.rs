use super::types::{Pool, PoolMap, TradePath};
use num_bigint::BigUint;
use num_traits::{CheckedSub, ConstZero, One, Zero};
use starknet::{
    core::types::{
        BlockId, BlockTag, EventFilter, Felt, FunctionCall, MaybePendingBlockWithTxHashes,
    },
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::ops::Mul;
use std::path::Path;
use std::str::FromStr;

const SCALE: f64 = 1000000 as f64;

fn create_pools_from_csv<P: AsRef<Path>>(path: P) -> io::Result<PoolMap> {
    let required_edges = [
        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
        "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
        "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
        "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
        "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
    ];

    let mut pool_map = PoolMap::new();
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();

        if parts.len() >= 3
            && required_edges.contains(&parts[1].as_str())
            && required_edges.contains(&parts[2].as_str())
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
                },
            );
        }
    }

    Ok(pool_map)
}

pub async fn get_latest_pool_data<P: AsRef<Path>>(
    required_trade_paths: Vec<TradePath>,
    path: P,
) -> PoolMap {
    let mut pool_map = create_pools_from_csv(path).unwrap();
    let mut required_paths: Vec<Vec<String>> = vec![];
    for trade_path in required_trade_paths {
        required_paths.push(trade_path.tokens.clone());
    }

    for path in required_paths {
        for pair in path.windows(2) {
            //
            let pool_key = if BigUint::parse_bytes(&pair[0].as_str()[2..].as_bytes(), 16).unwrap()
                < BigUint::parse_bytes(&pair[1].as_str()[2..].as_bytes(), 16).unwrap()
            {
                (pair[0].clone(), pair[1].clone())
            } else {
                (pair[1].clone(), pair[0].clone())
            };

            let possible_pool = pool_map.get(&pool_key);
            if possible_pool.is_some() {
                let pool = possible_pool.unwrap();
                if pool.reserves_updated {
                    continue;
                }
                let provider = JsonRpcClient::new(
        HttpTransport::new(
            Url::parse("https://rpc.nethermind.io/mainnet-juno?apikey=5n1kZyTyMGiYmPn5YtGxlwHYSFTDRGCTGTfzFIn8nGKMdyOa").unwrap()));
                let mut calldata = vec![];
                //let mut str_felts = vec![];
                let mut byte_felts = vec![];

                let result = provider
                    .call(
                        FunctionCall {
                            contract_address: Felt::from_hex(&pool.address).unwrap(),
                            entry_point_selector: Felt::from_hex(
                                "0x3cb0e1486e633fbe3e2fafe8aedf12b70ca1860e7467ddb75a17858cde39312",
                            )
                            .unwrap(),
                            calldata,
                        },
                        BlockId::Tag(BlockTag::Latest),
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
                };
                pool_map.insert(pool_key, updated_pool.clone());
            }
        }
    }
    pool_map
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
