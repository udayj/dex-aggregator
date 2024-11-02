use anyhow::Context;

use super::{
    types::Graph,
    Result
};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub fn compute_graph_from_csv<P: AsRef<Path>>(
    path: P,
    required_tokens: &[String],
) -> Result<Graph> {
    let mut graph = Graph::new();
    let file = File::open(path).context("Couldn't open file while trying to compute graph".to_string())?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();

        if parts.len() >= 3
            && required_tokens.contains(&parts[1])
            && required_tokens.contains(&parts[2])
        {
            // Ignore first column, use second and third
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
            .or_default()
            .push(to.to_string());

        // Add to -> from (since undirected)
        self.edges
            .entry(to.to_string())
            .or_default()
            .push(from.to_string());
    }

    pub fn find_all_paths(
        &self,
        start: &str,
        target_nodes: &HashSet<String>,
    ) -> HashMap<String, Vec<Vec<String>>> {
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
            target_nodes,
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
        target_nodes: &HashSet<String>,
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
                    self.dfs(
                        neighbor,
                        start,
                        visited,
                        current_path,
                        all_paths,
                        target_nodes,
                    );
                }
            }
        }

        visited.remove(current);
        current_path.pop();
    }
}
