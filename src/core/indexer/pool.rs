use super::types::{Pool, PoolMap};
use super::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn write_poolmap_data_on_disk<P: AsRef<Path>>(
    poolmap_file_path: P,
    pool_map: &HashMap<(String, String), Pool>,
) -> Result<()> {
    let pool_list = PoolList::from_hash_map(pool_map);
    let json = serde_json::to_string_pretty(&pool_list)?;

    fs::write(poolmap_file_path, json)?;
    Ok(())
}

pub fn read_poolmap_data_from_disk<P: AsRef<Path>>(poolmap_file_path: P) -> Result<PoolMap> {
    let pool_list_json = fs::read_to_string(poolmap_file_path)?;
    let pool_list: PoolList = serde_json::from_str(&pool_list_json)?;
    let pool_map = pool_list.to_hash_map();
    Ok(pool_map)
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
