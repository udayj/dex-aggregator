use super::pair_data::get_latest_pair_data;
use super::types::Quote;
use super::paths::{store_path_data_on_disk, get_paths_between};
use super::pool::get_latest_pool_data;
use super::optimization::{optimize_amount_out, optimize_amount_in};
use num_bigint::BigUint;
use super::types::DexConfig;

pub async fn get_aggregator_quotes(config: &DexConfig, params: Quote) {
    //get_latest_pair_data().await;
    //store_path_data_on_disk("all_pair_tokens.csv");
    let required_trade_paths = get_paths_between(params.sellTokenAddress, params.buyTokenAddress);

    let pool_map = get_latest_pool_data(required_trade_paths.clone(), "all_pair_tokens.csv").await;

    if params.buyAmount.is_some() {
        let (splits, total_amount) = optimize_amount_in(required_trade_paths.clone(), pool_map, BigUint::from(params.buyAmount.unwrap().parse::<u128>().unwrap()));
    println!("Total Amount In:{}\n Splits: {:?}\n", total_amount, splits);
    }
    else{
        let (splits, total_amount) = optimize_amount_out(required_trade_paths, pool_map, BigUint::from(params.sellAmount.unwrap().parse::<u128>().unwrap()));
        println!("Total Amount Out:{}\n Splits: {:?}\n", total_amount, splits);
    }
    
}
