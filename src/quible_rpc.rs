use jsonrpsee::proc_macros::rpc;
// use jsonrpsee::core::client::ClientT;
// use jsonrpsee::http_client::HttpClient;
// use jsonrpsee::rpc_params;
use jsonrpsee::types::ErrorObjectOwned;

use crate::types;

#[rpc(server, client, namespace = "quible")]
pub trait QuibleRpc {
    // for some reason the macro makes RpcServerServer
    #[method(name = "sendTransaction")]
    async fn send_transaction(
        &self,
        transaction: types::Transaction,
    ) -> Result<types::Transaction, ErrorObjectOwned>;

    #[method(name = "requestProof")]
    async fn request_proof(
        &self,
        quirkle_root: types::QuirkleRoot,
        member_address: String,
        requested_at_block_number: u128,
    ) -> Result<types::QuirkleProof, ErrorObjectOwned>;
}
