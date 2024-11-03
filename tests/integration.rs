use csv::Writer;
use dex_aggregator::core::constants::INFINITE;
use dex_aggregator::core::optimization::{optimize_amount_in, optimize_amount_out};
use dex_aggregator::core::path::get_paths_between;
use dex_aggregator::core::pool::get_indexed_pool_data;
use dex_aggregator::types::DexConfig;
use num_bigint::BigUint;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

#[test]
fn get_quotes_given_amount_in() {
    let mut config = DexConfig::default();
    config.working_dir = "tests/working_dir".to_string();
    let dir = Path::new(config.working_dir.as_str());
    let pathmap_file_path = dir.join(config.pathmap_file.clone());

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(config.working_dir.as_str());
   
    let symbol_list: Vec<(&str, &str)> = vec![
        (
            "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            "ETH",
        ),
        (
            "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
            "USDT",
        ),
        (
            "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
            "USDC",
        ),
        (
            "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
            "wstETH",
        ),
        (
            "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
            "DAI",
        ),
        (
            "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
            "STRK",
        ),
    ];

    let amount_in_list: Vec<(&str, &str)> = vec![
        (
            "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            "2601850195660800000",
        ),
        (
            "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
            "10000000000",
        ),
        (
            "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
            "10000000000",
        ),
        (
            "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
            "1557852794326010000",
        ),
        (
            "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
            "10000000000000000000000",
        ),
        (
            "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
            "21069000000000000000000",
        ),
    ];

    let symbol_map: HashMap<String, String> = symbol_list
        .iter()
        .map(|token| (token.0.to_string(), token.1.to_string()))
        .collect();

    let amount_in_map: HashMap<String, String> = amount_in_list
        .iter()
        .map(|token| (token.0.to_string(), token.1.to_string()))
        .collect();

    let output_file = dir.join("given_amount_in_test_runs.csv");
    let file = File::create(output_file).unwrap();
    let mut wrt = Writer::from_writer(file);
    let _ = wrt.write_record(["TOKEN IN", "TOKEN OUT", "AMOUNT IN", "AMOUNT OUT"]);
    let all_token_pairs = get_all_token_pairs(&config.supported_tokens);
    for pair in all_token_pairs.iter() {
        if pair.0.clone() == pair.1.clone() {
            continue;
        }

        let amount_in = amount_in_map.get(&pair.0).unwrap();
        let required_trade_paths =
            get_paths_between(pathmap_file_path.clone(), pair.0.clone(), pair.1.clone()).unwrap();

        let dir = Path::new(config.working_dir.as_str());
        let poolmap_file_path = dir.join(config.poolmap_file.clone());
        let (pool_map, _) = get_indexed_pool_data(poolmap_file_path).unwrap();

        let (_, total_amount) = optimize_amount_out(
            required_trade_paths.clone(),
            pool_map.clone(),
            BigUint::from(amount_in.parse::<u128>().unwrap()),
        );

        let _ = wrt.write_record([
            symbol_map.get(&pair.0).unwrap(),
            symbol_map.get(&pair.1).unwrap(),
            amount_in,
            &total_amount.to_string(),
        ]);
    }

    let _ = wrt.flush();
    /*let required_trade_paths = get_paths_between(
        pathmap_file_path,
        params.sellTokenAddress.clone(),
        params.buyTokenAddress.clone(),
    )
    .map_err(|e| anyhow!(format!("{}", e)))?;*/
}

#[test]
fn get_quotes_given_amount_out() {
    let mut config = DexConfig::default();
    config.working_dir = "tests/working_dir".to_string();
    let dir = Path::new(config.working_dir.as_str());
    let pathmap_file_path = dir.join(config.pathmap_file.clone());

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(config.working_dir.as_str());
    println!("{:?}", dir.as_os_str());
    
    let symbol_list: Vec<(&str, &str)> = vec![
        (
            "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            "ETH",
        ),
        (
            "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
            "USDT",
        ),
        (
            "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
            "USDC",
        ),
        (
            "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
            "wstETH",
        ),
        (
            "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
            "DAI",
        ),
        (
            "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
            "STRK",
        ),
    ];

    let amount_out_list: Vec<(&str, &str)> = vec![
        (
            "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            "2601850195660800000",
        ),
        (
            "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
            "10000000000",
        ),
        (
            "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
            "10000000000",
        ),
        (
            "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
            "1557852794326010000",
        ),
        (
            "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
            "10000000000000000000000",
        ),
        (
            "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
            "21069000000000000000000",
        ),
    ];

    let symbol_map: HashMap<String, String> = symbol_list
        .iter()
        .map(|token| (token.0.to_string(), token.1.to_string()))
        .collect();

    let amount_out_map: HashMap<String, String> = amount_out_list
        .iter()
        .map(|token| (token.0.to_string(), token.1.to_string()))
        .collect();

    let output_file = dir.join("given_amount_out_test_runs.csv");
    let file = File::create(output_file).unwrap();
    let mut wrt = Writer::from_writer(file);
    let _ = wrt.write_record(["TOKEN IN", "TOKEN OUT", "AMOUNT OUT", "AMOUNT IN"]);
    let all_token_pairs = get_all_token_pairs(&config.supported_tokens);
    for pair in all_token_pairs.iter() {
        if pair.0.clone() == pair.1.clone() {
            continue;
        }

        let amount_out = amount_out_map.get(&pair.1).unwrap();
        let required_trade_paths =
            get_paths_between(pathmap_file_path.clone(), pair.0.clone(), pair.1.clone()).unwrap();

        let dir = Path::new(config.working_dir.as_str());
        let poolmap_file_path = dir.join(config.poolmap_file.clone());
        let (pool_map, _) = get_indexed_pool_data(poolmap_file_path).unwrap();

        let (_, total_amount) = optimize_amount_in(
            required_trade_paths.clone(),
            pool_map.clone(),
            BigUint::from(amount_out.parse::<u128>().unwrap()),
        );
        if total_amount == INFINITE() {
            let _ = wrt.write_record([
            symbol_map.get(&pair.0).unwrap(),
            symbol_map.get(&pair.1).unwrap(),
            amount_out,
            &"INFINITE".to_string(),
        ]);
        }
        else {

            let _ = wrt.write_record([
            symbol_map.get(&pair.0).unwrap(),
            symbol_map.get(&pair.1).unwrap(),
            amount_out,
            &total_amount.to_string(),
        ]);
        }
        
    }

    let _ = wrt.flush();
    /*let required_trade_paths = get_paths_between(
        pathmap_file_path,
        params.sellTokenAddress.clone(),
        params.buyTokenAddress.clone(),
    )
    .map_err(|e| anyhow!(format!("{}", e)))?;*/
}

fn get_unique_token_pairs(strings: &[String]) -> Vec<(String, String)> {
    strings
        .iter()
        .enumerate()
        .flat_map(|(i, s1)| {
            strings[i + 1..]
                .iter()
                .map(move |s2| (s1.clone(), s2.clone()))
        })
        .collect()
}

fn get_all_token_pairs(strings: &[String]) -> Vec<(String, String)> {
    strings
        .iter()
        .enumerate()
        .flat_map(|(i, s1)| strings.iter().map(move |s2| (s1.clone(), s2.clone())))
        .collect()
}
