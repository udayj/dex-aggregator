use num_bigint::BigUint;
use num_traits::{CheckedSub, Zero};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str::FromStr;
use super::indexer::path_indexer::{
    read_pathmap_from_disk, read_token_paths, write_pathmap_on_disk, write_paths_to_file,
};
use super::token_graph::compute_graph_from_csv;
use super::types::{PathMap, Pool, TradePath};
use super::Result;

pub fn update_path_data<P: AsRef<Path>>(
    path: P,
    required_tokens: &Vec<String>,
    output_paths: &[P],
) -> Result<()> {
    let graph = compute_graph_from_csv(path, required_tokens)?;
    for (i, token) in required_tokens.iter().enumerate() {
        let start_node = token.to_string().clone();
        let mut target_nodes: HashSet<String> = vec![].into_iter().collect();
        for int_token in required_tokens {
            if int_token != token {
                target_nodes.insert(int_token.to_string());
            }
        }
        // Find all paths
        let paths = graph.find_all_paths(&start_node, target_nodes);

        // Write results to file using indexer
        write_paths_to_file(&paths, &output_paths[i])?;
    }
    Ok(())
}

pub fn get_all_paths<P: AsRef<Path>>(file_paths: &[P]) -> Result<PathMap> {
    let mut combined_map: PathMap = PathMap::new();

    for file_path in file_paths {
        let paths = read_token_paths(file_path)?;

        // Merge the new paths into the combined map
        for (key, mut paths_vec) in paths {
            combined_map
                .entry(key)
                .or_insert_with(Vec::new)
                .append(&mut paths_vec);
        }
    }

    Ok(combined_map)
}

pub fn update_pathmap<P: AsRef<Path>>(
    pathmap_file: P,
    output_paths: &[P],
) -> Result<()> {
    let path_map = get_all_paths(output_paths)?;

    write_pathmap_on_disk(pathmap_file, &path_map)?;
    Ok(())
}

pub fn get_paths_between<P: AsRef<Path>>(
    pathmap_file: P,
    token_in: String,
    token_out: String,
) -> Result<Vec<TradePath>> {
    let path_map = read_pathmap_from_disk(pathmap_file)?;
    let dummy_value = vec![vec![]];
    let required_paths = path_map.get(&(token_in, token_out)).unwrap_or(&dummy_value);
    let required_trade_paths: Vec<TradePath> = required_paths
        .iter()
        .map(|x| TradePath { tokens: x.clone() })
        .collect();

    Ok(required_trade_paths)
}

impl TradePath {
    pub fn get_amount_out(
        &self,
        amount_in: &BigUint,
        pools: &mut HashMap<(String, String), Pool>,
    ) -> BigUint {
        let mut current_amount = amount_in.clone();

        // Process each hop in the path
        for token_pair in self.tokens.windows(2) {
            let token_in = &token_pair[0];
            let token_out = &token_pair[1];

            // Create pool key (always order tokens lexicographically)
            let pool_key = if BigUint::parse_bytes(token_in.as_str()[2..].as_bytes(), 16).unwrap()
                < BigUint::parse_bytes(token_out.as_str()[2..].as_bytes(), 16).unwrap()
            {
                (token_in.clone(), token_out.clone())
            } else {
                (token_out.clone(), token_in.clone())
            };

            // Get pool and calculate output
            if let Some(pool) = pools.get(&pool_key) {
                if *token_in.clone() == pool_key.0 {
                    let current_amount_in = current_amount.clone();
                    current_amount = pool.get_amount_out(
                        &current_amount,
                        &pool.reserve0.clone(),
                        &pool.reserve1.clone(),
                    );
                    let updated_pool = Pool {
                        reserve0: pool.reserve0.clone() + current_amount_in,
                        reserve1: pool.reserve1.clone() - current_amount.clone(),
                        reserves_updated: true,
                        address: pool.address.clone(),
                        fee: pool.fee.clone(),
                        block_number: pool.block_number,
                    };
                    if current_amount.clone() == BigUint::zero() {
                        return BigUint::zero();
                    }
                    pools.insert(pool_key, updated_pool);
                } else {
                    let current_amount_in = current_amount.clone();
                    current_amount = pool.get_amount_out(
                        &current_amount,
                        &pool.reserve1.clone(),
                        &pool.reserve0.clone(),
                    );
                    let updated_pool = Pool {
                        reserve0: pool.reserve0.clone() - current_amount.clone(),
                        reserve1: pool.reserve1.clone() + current_amount_in,
                        reserves_updated: true,
                        address: pool.address.clone(),
                        fee: pool.fee.clone(),
                        block_number: pool.block_number,
                    };
                    if current_amount.clone() == BigUint::zero() {
                        return BigUint::zero();
                    }
                    pools.insert(pool_key, updated_pool);
                }
            } else {
                return BigUint::zero(); // Pool not found
            }
        }

        current_amount
    }

    pub fn get_amount_in(
        &self,
        amount_out: &BigUint,
        pools: &mut HashMap<(String, String), Pool>,
    ) -> BigUint {
        let mut current_amount = amount_out.clone();
        let tokens: Vec<String> = self.tokens.clone().into_iter().rev().collect();
        // Process each hop in the path
        for token_pair in tokens.windows(2) {
            let token_out = &token_pair[0];
            let token_in = &token_pair[1];

            // Create pool key (always order tokens lexicographically)
            let pool_key = if BigUint::parse_bytes(token_in.as_str()[2..].as_bytes(), 16).unwrap()
                < BigUint::parse_bytes(token_out.as_str()[2..].as_bytes(), 16).unwrap()
            {
                (token_in.clone(), token_out.clone())
            } else {
                (token_out.clone(), token_in.clone())
            };

            // Get pool and calculate output
            if let Some(pool) = pools.get(&pool_key) {
                if *token_in.clone() == pool_key.0 {
                    let current_amount_out = current_amount.clone();
                    current_amount = pool.get_amount_in(
                        &current_amount,
                        &pool.reserve0.clone(),
                        &pool.reserve1.clone(),
                    );
                    if current_amount.clone()
                        == BigUint::from_str("1000000000000000000000000000").unwrap()
                    {
                        return BigUint::from_str("1000000000000000000000000000").unwrap();
                    }
                    let updated_pool = Pool {
                        reserve0: pool.reserve0.clone() + current_amount.clone(),
                        reserve1: BigUint::checked_sub(
                            &pool.reserve1.clone(),
                            &current_amount_out.clone(),
                        )
                        .unwrap_or(BigUint::ZERO),
                        reserves_updated: true,
                        address: pool.address.clone(),
                        fee: pool.fee.clone(),
                        block_number: pool.block_number,
                    };

                    pools.insert(pool_key, updated_pool);
                } else {
                    let current_amount_out = current_amount.clone();
                    current_amount = pool.get_amount_in(
                        &current_amount,
                        &pool.reserve1.clone(),
                        &pool.reserve0.clone(),
                    );

                    if current_amount.clone()
                        == BigUint::from_str("1000000000000000000000000000").unwrap()
                    {
                        return BigUint::from_str("1000000000000000000000000000").unwrap();
                    }
                    let updated_pool = Pool {
                        reserve0: BigUint::checked_sub(
                            &pool.reserve0.clone(),
                            &current_amount_out.clone(),
                        )
                        .unwrap_or(BigUint::ZERO),
                        reserve1: pool.reserve1.clone() + current_amount.clone(),
                        reserves_updated: true,
                        address: pool.address.clone(),
                        fee: pool.fee.clone(),
                        block_number: pool.block_number,
                    };

                    pools.insert(pool_key, updated_pool);
                }
            } else {
                return BigUint::zero(); // Pool not found
            }
        }
        current_amount
    }
}
