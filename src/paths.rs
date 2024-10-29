use super::token_graph::compute_graph_from_csv;
use super::types::{Graph, PathMap, Pool, TokenPath, TradePath};
use num_bigint::BigUint;
use num_traits::{CheckedSub, ConstZero, One, Zero};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::ops::Mul;
use std::path::Path;
use std::str::FromStr;

pub fn store_path_data_on_disk<P: AsRef<Path>>(path: P) {
    let graph = compute_graph_from_csv(path).unwrap();
    let required_nodes = [
        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
        "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
        "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
        "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
        "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
    ];

    for edge in required_nodes {
        let start_node = edge.to_string().clone();
        let mut target_nodes: HashSet<String> = vec![].into_iter().collect();
        for int_edge in required_nodes {
            if int_edge != edge {
                target_nodes.insert(int_edge.to_string());
            }
        }
        // Find all paths
        let paths = graph.find_all_paths(&start_node, target_nodes);

        // Write results to file
        write_paths_to_file(
            &start_node,
            &paths,
            format!("{}.txt", edge.to_string()).as_str(),
        )
        .unwrap();
    }
}

fn write_paths_to_file(
    start_node: &str,
    paths: &HashMap<String, Vec<Vec<String>>>,
    output_path: &str,
) -> io::Result<()> {
    let mut file = File::create(output_path)?;

    for (destination, path_list) in paths.iter() {
        //writeln!(file, "\nPaths to {}:", destination)?;
        let mut new_path_list = path_list.clone();
        new_path_list.sort_by(|a, b| a.len().cmp(&b.len()));
        for (i, path) in new_path_list.iter().enumerate() {
            writeln!(file, "{}", path.join(" "))?;
        }
    }
    Ok(())
}

fn read_token_paths<P: AsRef<Path>>(file_path: P) -> io::Result<PathMap> {
    // Create a HashMap to store paths indexed by (start_token, end_token)
    let mut path_map: PathMap = HashMap::new();

    // Open the file
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    // Process each line
    for line in reader.lines() {
        let line = line?;

        // Split the line by whitespace
        let tokens: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();

        // Skip invalid lines
        if tokens.len() < 2 {
            continue;
        }

        // Extract start and end tokens
        let start_token = tokens[0].clone();
        let end_token = tokens[tokens.len() - 1].clone();

        // Create the path key
        let path_key = (start_token.clone(), end_token.clone());

        // Add the path to the map
        path_map
            .entry(path_key)
            .or_insert_with(Vec::new)
            .push(tokens);
    }

    Ok(path_map)
}

// Function to process multiple token files
pub fn get_all_paths(file_paths: &Vec<String>) -> io::Result<PathMap> {
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

pub fn get_paths_between(token_in: String, token_out: String) -> Vec<TradePath> {
    let required_edges = [
        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
        "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
        "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
        "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
        "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
    ];

    let path_map = get_all_paths(
        &(required_edges
            .iter()
            .map(|x| format!("{}.txt", x).to_string())
            .collect()),
    )
    .unwrap();
    let dummy_value = vec![vec![]];
    let required_paths = path_map.get(&(token_in, token_out)).unwrap_or(&dummy_value);
    let mut required_trade_paths: Vec<TradePath> = required_paths
        .iter()
        .map(|x| TradePath { tokens: x.clone() })
        .collect();

    required_trade_paths
}

impl TradePath {
    fn new(path_str: &str) -> Self {
        TradePath {
            tokens: path_str.split_whitespace().map(String::from).collect(),
        }
    }

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
            let pool_key = if BigUint::parse_bytes(&token_in.as_str()[2..].as_bytes(), 16).unwrap()
                < BigUint::parse_bytes(&token_out.as_str()[2..].as_bytes(), 16).unwrap()
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
            let pool_key = if BigUint::parse_bytes(&token_in.as_str()[2..].as_bytes(), 16).unwrap()
                < BigUint::parse_bytes(&token_out.as_str()[2..].as_bytes(), 16).unwrap()
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
