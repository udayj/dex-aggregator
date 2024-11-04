# Dex Aggregator for Jediswap v1 pools

This is a lightweight dex aggregator. It exposes a simple REST based API service to get trade quotes for a token pair and manage the underlying datastore.

To compile `cargo build --release` (using rust version 1.80.0)

To run `cargo run --release` - dex_config.toml holds configurable values

Server will be available at http://localhost:3000

Sample query to get [trade quotes](http://localhost:3000/quotes?sellTokenAddress=0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8&buyTokenAddress=0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d&sellAmount=10000000000&getLatest=true) - 
The `getLatest` query parameter lets you use pre indexed data (if false or not present) and get latest reserves data (if true)

Open Api Docs available at http://localhost:3000/api-docs/openapi.json

Swagger UI available at http://localhost:3000/swagger-ui/

The basic idea is to find all possible paths to trade between a token pair and then do gradient optimisation to optimise an objective function - maximise output when given an input amount and minimise input when given an output amount. Ref [here](https://github.com/udayj/dex-aggregator/tree/main/notes) for some math notes.

