use super::optimization::{optimize_amount_in, optimize_amount_out};
use super::pair_data::get_latest_pair_data;
use super::paths::{get_paths_between, store_path_data_on_disk, store_pathmap_on_disk};
use super::pool::get_latest_pool_data;
use super::types::DexConfig;
use super::types::Quote;
use num_bigint::BigUint;
use std::path::Path;
use std::fs;
use std::error::Error;

pub async fn update_and_save_pair_data(config: &DexConfig) -> Result<(), Box<dyn Error>>{
    if !Path::new(config.working_dir.as_str()).exists() {
        fs::create_dir(config.working_dir.clone())?;
    }

    let dir = Path::new(config.working_dir.as_str());
    let pair_file_path = dir.join(config.pair_file.clone());
    let dir = Path::new(config.working_dir.as_str());
    let token_pair_file_path = dir.join(config.token_pair_file.clone());

    get_latest_pair_data(
        &config.rpc_url, pair_file_path.to_str().unwrap(), token_pair_file_path.to_str().unwrap()).await?;
    Ok(())
}

pub async fn update_and_save_path_data(config: &DexConfig) -> Result<(), Box<dyn Error>> { 
    if !Path::new(config.working_dir.as_str()).exists() {
        return Err("Token Pair data file not found".into());
    }
    let dir = Path::new(config.working_dir.as_str());
    let token_pair_file_path = dir.join(config.token_pair_file.clone());

    let dir = Path::new(config.working_dir.as_str());
    let pathmap_file_path = dir.join(config.pathmap_file.clone());
    
    let mut output_paths = vec![];
    for token in &config.supported_tokens {
        let dir = Path::new(config.working_dir.as_str());
        let token_paths_file = dir.join(token.clone()+ ".txt");   
        output_paths.push(token_paths_file);
    }
    store_path_data_on_disk(token_pair_file_path, &config.supported_tokens, &output_paths)?;
    store_pathmap_on_disk(pathmap_file_path, &output_paths)?;
    Ok(())
}

pub async fn get_aggregator_quotes(config: &DexConfig, params: Quote) -> Result<(), Box<dyn Error>>{
    
    if !Path::new(config.working_dir.as_str()).exists() {
        return Err("Token Path data files not found".into());
    }
    let dir = Path::new(config.working_dir.as_str());
    let pathmap_file_path = dir.join(config.pathmap_file.clone());

    let required_trade_paths = get_paths_between(pathmap_file_path, params.sellTokenAddress, params.buyTokenAddress)?;

    let pool_map = get_latest_pool_data(required_trade_paths.clone(), "all_pair_tokens.csv").await;

    if params.buyAmount.is_some() {
        let (splits, total_amount) = optimize_amount_in(
            required_trade_paths.clone(),
            pool_map,
            BigUint::from(params.buyAmount.unwrap().parse::<u128>().unwrap()),
        );
        println!("Total Amount In:{}\n Splits: {:?}\n", total_amount, splits);
    } else {
        let (splits, total_amount) = optimize_amount_out(
            required_trade_paths,
            pool_map,
            BigUint::from(params.sellAmount.unwrap().parse::<u128>().unwrap()),
        );
        println!("Total Amount Out:{}\n Splits: {:?}\n", total_amount, splits);
    }
    Ok(())
}
