use super::types::PathMap;
use super::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

pub fn write_paths_to_file<P: AsRef<Path>>(
    paths: &HashMap<String, Vec<Vec<String>>>,
    output_path: &P,
) -> Result<()> {
    let mut file = File::create(output_path)?;

    for (_, path_list) in paths.iter() {
        //writeln!(file, "\nPaths to {}:", destination)?;
        let mut new_path_list = path_list.clone();
        new_path_list.sort_by_key(|a| a.len());
        for path in new_path_list.iter() {
            writeln!(file, "{}", path.join(" "))?;
        }
    }
    Ok(())
}

pub fn read_token_paths<P: AsRef<Path>>(file_path: P) -> Result<PathMap> {
    // Create a HashMap to store paths indexed by (start_token, end_token)
    let mut path_map: PathMap = HashMap::new();

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

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
        path_map.entry(path_key).or_default().push(tokens);
    }

    Ok(path_map)
}

pub fn write_pathmap_on_disk<P: AsRef<Path>>(
    pathmap_file: P,
    path_map: &HashMap<(String, String), Vec<Vec<String>>>,
) -> Result<()> {
    let path_list = PathList::from_hash_map(path_map);
    let json = serde_json::to_string_pretty(&path_list)?;

    fs::write(pathmap_file, json)?;
    Ok(())
}

pub fn read_pathmap_from_disk<P: AsRef<Path>>(pathmap_file: P) -> Result<PathMap> {
    let path_list_json = fs::read_to_string(pathmap_file)?;
    let path_list: PathList = serde_json::from_str(&path_list_json)?;
    let path_map = path_list.to_hash_map();
    Ok(path_map)
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
