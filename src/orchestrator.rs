use super::optimization::{optimize_amount_in, optimize_amount_out};
use super::pair_data::get_latest_pair_data;
use super::paths::{get_paths_between, store_path_data_on_disk};
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

pub async fn get_aggregator_quotes(config: &DexConfig, params: Quote) -> Result<(), Box<dyn Error>>{
    
    //store_path_data_on_disk("all_pair_tokens.csv");
    let required_trade_paths = get_paths_between(params.sellTokenAddress, params.buyTokenAddress);

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
