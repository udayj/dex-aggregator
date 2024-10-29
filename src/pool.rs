use super::types::{PoolMap, Pool, TradePath};
use std::path::Path;
use std::fs::File;
use num_bigint::BigUint;
use std::io::{self, BufReader, BufRead};
use starknet::{
    core::types::{BlockId, BlockTag, EventFilter, Felt, FunctionCall, MaybePendingBlockWithTxHashes},
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};

fn create_pools_from_csv<P: AsRef<Path>>(path: P) -> io::Result<PoolMap> {

    let required_edges = [
        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
        "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
        "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
        "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
        "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d"
    ];

    let mut pool_map = PoolMap::new();
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<String> = line.split(',')
            .map(|s| s.trim().to_string())
            .collect();
        
        if parts.len() >= 3 && required_edges.contains(&parts[1].as_str()) && required_edges.contains(&parts[2].as_str()){  
            let (token0, token1) = if BigUint::parse_bytes(&parts[1].as_str()[2..].as_bytes(), 16).unwrap() < BigUint::parse_bytes(&parts[2].as_str()[2..].as_bytes(), 16).unwrap() {
                (parts[1].clone(), parts[2].clone())
            } else {
                (parts[2].clone(), parts[1].clone())
            };
           pool_map.insert((token0, token1), Pool {
            address: parts[0].clone(),
            reserve0: BigUint::ZERO,
            reserve1: BigUint::ZERO,
            fee: BigUint::ZERO,
            reserves_updated: false
           });
        }
    }

    Ok(pool_map)
}

pub async fn get_latest_pool_data<P: AsRef<Path>>(required_trade_paths: Vec<TradePath>, path: P) -> 
PoolMap {

    let mut pool_map = create_pools_from_csv(path).unwrap();
    let mut required_paths: Vec<Vec<String>> = vec![];
    for trade_path in required_trade_paths {

        required_paths.push(trade_path.tokens.clone());
    }

    for path in required_paths {
        for pair in path.windows(2) {

            // 
            let pool_key = if BigUint::parse_bytes(&pair[0].as_str()[2..].as_bytes(), 16).unwrap() < BigUint::parse_bytes(&pair[1].as_str()[2..].as_bytes(), 16).unwrap() {
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
            
            let result = provider.call(
                FunctionCall {
                    contract_address: Felt::from_hex(
                        &pool.address).unwrap(),
                    entry_point_selector: Felt::from_hex("0x3cb0e1486e633fbe3e2fafe8aedf12b70ca1860e7467ddb75a17858cde39312").unwrap(),
                    calldata
                }, BlockId::Tag(BlockTag::Latest)).await.unwrap();
            
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
                fee: pool.fee.clone()
            };
            pool_map.insert(pool_key, updated_pool.clone());
            
            }
        }
    }
    pool_map
}   