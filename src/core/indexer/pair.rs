use super::constants::{FACTORY_ADDRESS, GET_ALL_PAIRS_SELECTOR, TOKEN0_SELECTOR, TOKEN1_SELECTOR};
use super::Result;
use csv::Writer;
use starknet::{
    core::types::{BlockId, BlockTag, Felt, FunctionCall},
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub async fn index_pair_data(rpc_url: &str, pair_file: &str, token_pair_file: &str) -> Result<()> {
    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url).unwrap()));

    let calldata = vec![];
    let rpc_url = rpc_url.to_string();
    // the following call gets list of all pairs
    let mut list_of_pairs = provider
        .call(
            FunctionCall {
                contract_address: Felt::from_hex(FACTORY_ADDRESS)?,
                entry_point_selector: Felt::from_hex(GET_ALL_PAIRS_SELECTOR)?,
                calldata,
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await?;

    let path = Path::new(pair_file);

    if !path.exists() {
        let file = File::create(pair_file).unwrap();
        let mut wrt = Writer::from_writer(file);
        let hex_pairs: Vec<String> = list_of_pairs
            .iter()
            .map(|felt| felt.to_hex_string())
            .collect();
        // 1st item is length
        hex_pairs
            .iter()
            .take(hex_pairs.len())
            .skip(1)
            .try_for_each(|record| wrt.write_record([record]))?;

        wrt.flush().unwrap();
    }

    /*
    FOR RECORD ONLY
    // This approach might not work since one of the constructor calldata is fee_to_setter which could be different
    // for different pairs of token deployments
    let file = File::create("all_pair_tokens.csv").unwrap();
    let mut wrt = Writer::from_writer(file);
    let mut sorted_required_edges = required_edges.clone();
    sorted_required_edges.sort_by(|a,b| a.cmp(&b));

    for i in 0..6 {
        for j in i+1..6 {

            let token0 = Felt::from_hex(sorted_required_edges[i]).unwrap();
            let token1 = Felt::from_hex(sorted_required_edges[j]).unwrap();
            let pair_address = calculate_contract_address(
                compute_hash_on_elements(&[token0, token1]),
                Felt::from_hex("0x07b5cd6a6949cc1730f89d795f2442f6ab431ea6c9a5be00685d50f97433c5eb").unwrap(),
                &[token0, token1]);
            wrt.write_record(&[
            pair_address.to_hex_string(),
            token0.to_hex_string(),
            token1.to_hex_string()
        ]);
        }
    }*/
    let file = File::create(token_pair_file).unwrap();
    let mut wrt = Writer::from_writer(file);
    list_of_pairs = list_of_pairs
        .clone()
        .into_iter()
        .take(list_of_pairs.len())
        .skip(1)
        .collect();
    let output: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(vec![]));
    // consider getting batch size from config
    for batch in list_of_pairs.chunks(50) {
        let mut threads = vec![];
        let batch_owned = batch.to_vec();
        for pair in batch_owned {
            let shared_output = output.clone();
            let rpc_url = rpc_url.clone();
            let worker_thread = tokio::spawn(async move {
                let provider =
                    JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url.as_str()).unwrap()));
                let calldata = vec![];

                let token0 = provider
                    .call(
                        FunctionCall {
                            contract_address: pair,
                            entry_point_selector: Felt::from_hex(TOKEN0_SELECTOR).unwrap(),
                            calldata,
                        },
                        BlockId::Tag(BlockTag::Latest),
                    )
                    .await
                    .unwrap();

                let token0 = token0[0].to_hex_string();
                let calldata = vec![];
                let token1 = provider
                    .call(
                        FunctionCall {
                            contract_address: pair,
                            entry_point_selector: Felt::from_hex(TOKEN1_SELECTOR).unwrap(),
                            calldata,
                        },
                        BlockId::Tag(BlockTag::Latest),
                    )
                    .await
                    .unwrap();
                let token1 = token1[0].to_hex_string();
                let token_pair: Vec<String> = vec![pair.to_hex_string(), token0, token1];
                let mut shared_output = shared_output.lock().unwrap();
                shared_output.push(token_pair);
            });
            threads.push(worker_thread);
        }

        for thread in threads.iter_mut() {
            thread.await.unwrap();
        }
    }

    for token_pair in output.lock().unwrap().iter() {
        wrt.write_record(token_pair)?;
    }
    wrt.flush()?;

    Ok(())
}
