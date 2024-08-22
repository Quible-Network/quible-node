use jsonrpsee::proc_macros::rpc;
// use jsonrpsee::core::client::ClientT;
// use jsonrpsee::http_client::HttpClient;
// use jsonrpsee::rpc_params;
use jsonrpsee::types::ErrorObjectOwned;

use crate::types;

#[rpc(server, client, namespace = "quible")]
pub trait QuibleRpc {
    // for some reason the macro makes RpcServerServer
    #[method(name = "quible_sendTransaction")]
    async fn send_transaction(
        &self,
        transaction: types::Transaction,
    ) -> Result<types::Transaction, ErrorObjectOwned>;

    #[method(name = "quible_requestProof")]
    async fn request_proof(
        &self,
        // TODO: define a type with custom serialize and deserialize
        //       logic that we can use for uint160 addresses
        quirkle_root: String,
        member_address: String,
        requested_at_block_number: u128,
    ) -> Result<types::QuirkleProof, ErrorObjectOwned>;

    // TODO: request proof
    // TODO: get block number
    // TODO: ...
}
