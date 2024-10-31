use super::types::DexConfig;
use std::path::PathBuf;
use confy;
use std::error::Error;
impl Default for DexConfig {
    fn default() -> Self {
        Self {
            working_dir: "working_dir".to_string(),
            pair_file: "pairs.csv".to_string(),
            token_pair_file: "all_token_pairs.csv".to_string(),
            supported_tokens: [
        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        "0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8",
        "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
        "0x42b8f0484674ca266ac5d08e4ac6a3fe65bd3129795def2dca5c34ecc5f96d2",
        "0x5574eb6b8789a91466f902c380d978e472db68170ff82a5b650b95a58ddf4ad",
        "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
    ].iter().map(|x| x.to_string()).collect(),
            pathmap_file: "pathmap.txt".to_string(),
            poolmap_file: "poolmap.txt".to_string()
        }
    }
}

impl DexConfig {

    // Helper method to load from a specific path
    pub fn load_from(path: PathBuf) -> Result<Self, Box<dyn Error>> {
        let config: Self = confy::load_path(path)?;
        Ok(config)
    }
}