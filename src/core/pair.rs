use super::indexer::pair_indexer::index_pair_data;
use super::Result;

pub async fn index_latest_pair_data(
    rpc_url: &str,
    pair_file: &str,
    token_pair_file: &str,
) -> Result<()> {
    index_pair_data(rpc_url, pair_file, token_pair_file).await
}
