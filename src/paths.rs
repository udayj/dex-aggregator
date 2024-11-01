use super::token_graph::compute_graph_from_csv;
use super::types::{PathKey, PathMap, Pool, TradePath};
use num_bigint::BigUint;
use num_traits::{CheckedSub, Zero};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::str::FromStr;

pub fn store_path_data_on_disk<P: AsRef<Path>>(
    path: P,
    required_tokens: &Vec<String>,
    output_paths: &[P],
) -> Result<(), Box<dyn Error>> {
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

        // Write results to file

        write_paths_to_file(&paths, &output_paths[i])?;
    }
    Ok(())
}

fn write_paths_to_file<P: AsRef<Path>>(
    paths: &HashMap<String, Vec<Vec<String>>>,
    output_path: &P,
) -> io::Result<()> {
    let mut file = File::create(output_path)?;

    for (_, path_list) in paths.iter() {
        //writeln!(file, "\nPaths to {}:", destination)?;
        let mut new_path_list = path_list.clone();
        new_path_list.sort_by(|a, b| a.len().cmp(&b.len()));
        for path in new_path_list.iter() {
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
pub fn get_all_paths<P: AsRef<Path>>(file_paths: &[P]) -> io::Result<PathMap> {
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

pub fn store_pathmap_on_disk<P: AsRef<Path>>(
    pathmap_file: P,
    output_paths: &[P],
) -> Result<(), Box<dyn Error>> {
    let path_map = get_all_paths(output_paths)?;

    let path_list = PathList::from_hash_map(&path_map);
    let json = serde_json::to_string_pretty(&path_list)?;

    fs::write(pathmap_file, json)?;
    Ok(())
}

pub fn get_paths_between<P: AsRef<Path>>(
    pathmap_file: P,
    token_in: String,
    token_out: String,
) -> Result<Vec<TradePath>, Box<dyn Error>> {
    let path_list_json = fs::read_to_string(pathmap_file)?;
    let path_list: PathList = serde_json::from_str(&path_list_json)?;
    let path_map = path_list.to_hash_map();
    let dummy_value = vec![vec![]];
    let required_paths = path_map.get(&(token_in, token_out)).unwrap_or(&dummy_value);
    let required_trade_paths: Vec<TradePath> = required_paths
        .iter()
        .map(|x| TradePath { tokens: x.clone() })
        .collect();

    Ok(required_trade_paths)
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

// The following is required to serialize and store pathmap as a json on disk
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PathList {
    // Use a vector of entries instead of HashMap with tuple keys
    paths: Vec<PathEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PathEntry {
    from: String,
    to: String,
    routes: Vec<Vec<String>>,
}

impl PathList {
    fn new() -> Self {
        Self { paths: Vec::new() }
    }

    // Convert from HashMap to the serializable structure
    fn from_hash_map(map: &HashMap<(String, String), Vec<Vec<String>>>) -> Self {
        let paths = map
            .iter()
            .map(|((from, to), routes)| PathEntry {
                from: from.clone(),
                to: to.clone(),
                routes: routes.clone(),
            })
            .collect();

        Self { paths }
    }

    // Convert back to HashMap
    fn to_hash_map(&self) -> HashMap<(String, String), Vec<Vec<String>>> {
        self.paths
            .iter()
            .map(|entry| ((entry.from.clone(), entry.to.clone()), entry.routes.clone()))
            .collect()
    }
}
