
use super::types::Graph;
use std::path::Path;
use std::fs::File;
use std::io::{self, BufReader, BufRead, Write};
use std::collections::{HashMap, HashSet};

pub fn compute_graph_from_csv<P: AsRef<Path>>(path: P) -> io::Result<Graph> {
    let mut graph = Graph::new();
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let required_edges = [
        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
        "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
        "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
        "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
        "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d"
    ];

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<String> = line.split(',')
            .map(|s| s.trim().to_string())
            .collect();
        
        if parts.len() >= 3 && required_edges.contains(&parts[1].as_str()) && required_edges.contains(&parts[2].as_str()) {  // Ignore first column, use second and third
            graph.add_edge(&parts[1], &parts[2]);
        }
    }

    Ok(graph)
}

impl Graph {
    fn new() -> Self {
        Graph {
            edges: HashMap::new(),
        }
    }

    // Add edge for both directions since graph is undirected
    fn add_edge(&mut self, from: &str, to: &str) {
        // Add from -> to
        self.edges
            .entry(from.to_string())
            .or_insert_with(Vec::new)
            .push(to.to_string());
        
        // Add to -> from (since undirected)
        self.edges
            .entry(to.to_string())
            .or_insert_with(Vec::new)
            .push(from.to_string());
    }

    pub fn find_all_paths(&self, start: &str, target_nodes: HashSet<String>) -> HashMap<String, Vec<Vec<String>>> {
        let mut all_paths: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        let mut visited = HashSet::new();
        let mut current_path = Vec::new();
        
        // Initialize paths for all nodes
        for node in target_nodes.iter() {
            all_paths.insert(node.clone(), Vec::new());
        }

        self.dfs(
            start,
            start,
            &mut visited,
            &mut current_path,
            &mut all_paths,
            &target_nodes
        );

        all_paths
    }

    fn dfs(
        &self,
        current: &str,
        start: &str,
        visited: &mut HashSet<String>,
        current_path: &mut Vec<String>,
        all_paths: &mut HashMap<String, Vec<Vec<String>>>,
        target_nodes: &HashSet<String>
    ) {
        if current_path.len() > 4 {
            return;
        }
        visited.insert(current.to_string());
        current_path.push(current.to_string());

        // If we're not at the start node, record this path
        if target_nodes.contains(current) && current != start {
            if let Some(paths) = all_paths.get_mut(current) {
                paths.push(current_path.clone());
            }
        }

        // Explore all neighbors
        if let Some(neighbors) = self.edges.get(current) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) && target_nodes.contains(neighbor) {
                    self.dfs(neighbor, start, visited, current_path, all_paths, target_nodes);
                }
            }
        }

        visited.remove(current);
        current_path.pop();
    }
}