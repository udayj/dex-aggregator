use utoipa::{ToSchema, IntoParams};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, ToSchema, IntoParams, Clone)]
pub struct Quote {
    #[schema(example = "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8")]
    sellTokenAddress: String,

    #[schema(example = "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d")]
    buyTokenAddress: String,

    #[schema(example = "1000000", nullable = true,)]
    sellAmount: Option<String>,

    #[schema(example = "2106900000", nullable = true,)]
    buyAmount: Option<String>
}
