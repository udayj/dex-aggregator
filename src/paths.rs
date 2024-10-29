use super::token_graph::compute_graph_from_csv;
use super::types::{Graph, TokenPath, PathMap, TradePath};
use std::path::Path;
use std::fs::File;
use std::io::{self, BufReader, BufRead, Write};
use std::collections::{HashMap, HashSet};

pub fn store_path_data_on_disk<P: AsRef<Path>>(path: P) {

    let graph = compute_graph_from_csv(path).unwrap();
    let required_nodes = [
        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
        "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
        "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
        "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
        "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d"
    ];

    for edge in required_nodes {

        let start_node = edge.to_string().clone();
        let mut target_nodes: HashSet<String> = vec![
        
        ].into_iter().collect();
        for int_edge in required_nodes {
            if int_edge != edge {
                target_nodes.insert(int_edge.to_string());
            }
        }
        // Find all paths
        let paths = graph.find_all_paths(&start_node, target_nodes);
        
        // Write results to file
        write_paths_to_file(&start_node, &paths, format!("{}.txt",edge.to_string()).as_str()).unwrap();


    }
}

fn write_paths_to_file(start_node: &str, paths: &HashMap<String, Vec<Vec<String>>>, output_path: &str) -> io::Result<()> {
    let mut file = File::create(output_path)?;
    
    for (destination, path_list) in paths.iter() {
        //writeln!(file, "\nPaths to {}:", destination)?;
        let mut new_path_list = path_list.clone();
        new_path_list.sort_by(|a,b| a.len().cmp(&b.len()));
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
        let tokens: Vec<String> = line
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        
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
        "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d"
    ];

    let path_map = get_all_paths(&(required_edges.iter().map(|x| format!("{}.txt",x).to_string()).collect())).unwrap();
    let dummy_value = vec![vec![]];
    let required_paths = path_map.get(&(token_in, token_out)).unwrap_or(&dummy_value);
    let mut required_trade_paths: Vec<TradePath> = required_paths.iter().map(|x| TradePath {tokens: x.clone()}).collect();

    required_trade_paths

}
