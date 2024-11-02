use super::constants::{FACTORY_ADDRESS, GET_ALL_PAIRS_SELECTOR, TOKEN0_SELECTOR, TOKEN1_SELECTOR};
use csv::Writer;
use starknet::{
    core::types::{BlockId, BlockTag, Felt, FunctionCall},
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
/*
THIS CODE IS ONLY NEEDED WHEN DETERMINISTIC PAIR ADDRESS IS COMPUTED
const CONTRACT_ADDRESS_PREFIX: Felt =
    Felt::from_hex_unchecked("0x535441524b4e45545f434f4e54524143545f41444452455353");

const ADDR_BOUND: NonZeroFelt = NonZeroFelt::from_raw([
    576459263475590224,
    18446744073709255680,
    160989183,
    18446743986131443745,
]);

fn calculate_contract_address(salt: Felt, class_hash: Felt, constructor_calldata: &[Felt]) -> Felt {
    compute_hash_on_elements(&[
        CONTRACT_ADDRESS_PREFIX,
        Felt::from_hex(
                "0x00dad44c139a476c7a17fc8141e6db680e9abc9f56fe249a105094c44382c2fd").unwrap(),
        salt,
        class_hash,
        compute_hash_on_elements(constructor_calldata),
    ])
    .mod_floor(&ADDR_BOUND)

    ANOTHER ALTERNATIVE TO GET TOKEN PAIR/POOLS IS TO ASK FACTORY/ROUTER WHETHER A POOL EXISTS FOR A PAIR
}*/

pub async fn get_latest_pair_data(
    rpc_url: &str,
    pair_file: &str,
    token_pair_file: &str,
) -> Result<(), Box<dyn Error>> {
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
        /*for pair in 1..1037 {
            wrt.write_record(&[hex_pairs[pair].as_str()])?;
        }*/
        wrt.flush().unwrap();
    }

    list_of_pairs = list_of_pairs[..13].to_vec();
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
    let mut output: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(vec![]));
    // consider getting batch size from config
    for batch in list_of_pairs.chunks(50) {
        let mut threads = vec![];
        let batch_owned = batch.to_vec();
        for pair in batch_owned {
            let mut shared_output = output.clone();
            let rpc_url = rpc_url.clone();
            let worker_thread = tokio::spawn(async move {
                let provider =
                    JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url.as_str()).unwrap()));
                let calldata = vec![];
                //println!("{}",result[pair].to_hex_string());
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
