use super::types::Quote;
use super::pair_data::get_latest_pair_data;
pub async fn get_aggregator_quotes(params: Quote) {

    get_latest_pair_data().await;

}