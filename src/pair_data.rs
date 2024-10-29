use csv::Writer;
use starknet::{
    core::crypto::compute_hash_on_elements,
    core::types::{
        BlockId, BlockTag, EventFilter, Felt, FunctionCall, MaybePendingBlockWithTxHashes,
        NonZeroFelt,
    },
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::{f32::consts::E, fs::File, str::FromStr};

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
}*/

pub async fn get_latest_pair_data() {
    // TODO: read all hard coded strings from config
    // TODO: get data using mpsc channels
    let provider = JsonRpcClient::new(
        HttpTransport::new(
            Url::parse("https://rpc.nethermind.io/mainnet-juno?apikey=5n1kZyTyMGiYmPn5YtGxlwHYSFTDRGCTGTfzFIn8nGKMdyOa").unwrap()));
    let required_edges = [
        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
        "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
        "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
        "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
        "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
    ];
    let mut calldata = vec![];
    let mut str_felts = vec![];
    // the following call gets list of all pairs
    let result = provider
        .call(
            FunctionCall {
                contract_address: Felt::from_hex(
                    "0x00dad44c139a476c7a17fc8141e6db680e9abc9f56fe249a105094c44382c2fd",
                )
                .unwrap(),
                entry_point_selector: Felt::from_hex(
                    "0x3e415d1aae9ddb9b1ffdb1f3bb6591b593e0a09748f635cdd067a74aba6f671",
                )
                .unwrap(),
                calldata,
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .unwrap();

    for item in result.clone() {
        str_felts.push(item.to_hex_string());
    }

    let path = Path::new("pairs.csv");

    if (!path.exists()) {
        let file = File::create("pairs.csv").unwrap();
        let mut wrt = Writer::from_writer(file);
        let hex_pairs: Vec<String> = result.iter().map(|felt| felt.to_hex_string()).collect();
        // 1st item is length
        for pair in 1..1037 {
            wrt.write_record(&[hex_pairs[pair].as_str()]);
        }
        wrt.flush().unwrap();
    }

    /*
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
    }

    wrt.flush();*/
    let file = File::create("all_pair_tokens.csv").unwrap();
    let mut wrt = Writer::from_writer(file);
    for pair in 1..result.len() {
        let mut calldata = vec![];
        //println!("{}",result[pair].to_hex_string());
        let result_pair = provider
            .call(
                FunctionCall {
                    contract_address: result[pair],
                    entry_point_selector: Felt::from_hex(
                        "0xad5d3ec16e143a33da68c00099116ef328a882b65607bec5b2431267934a20",
                    )
                    .unwrap(),
                    calldata,
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .unwrap();

        let token0 = result_pair[0].to_hex_string();
        let mut calldata = vec![];
        let result_pair = provider
            .call(
                FunctionCall {
                    contract_address: result[pair],
                    entry_point_selector: Felt::from_hex(
                        "0x3610e8e1835afecdd154863369b91f55612defc17933f83f4425533c435a248",
                    )
                    .unwrap(),
                    calldata,
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .unwrap();
        let token1 = result_pair[0].to_hex_string();
        wrt.write_record(&[result[pair].to_hex_string(), token0, token1]);
    }
    wrt.flush();
}
