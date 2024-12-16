use jsonrpsee::proc_macros::rpc;
// use jsonrpsee::core::client::ClientT;
// use jsonrpsee::http_client::HttpClient;
// use jsonrpsee::rpc_params;
use jsonrpsee::types::ErrorObjectOwned;

use crate::cert;
use crate::tx::types::Transaction;
use crate::types::{
    self, BlockDetailsPayload, BlockHeightPayload, FaucetOutputPayload, OutpointsPayload,
    ValueOutputsPayload,
};

#[rpc(server, client, namespace = "quible")]
pub trait QuibleRpc {
    // for some reason the macro makes RpcServerServer
    #[method(name = "sendTransaction")]
    async fn send_transaction(&self, transaction: Transaction) -> Result<(), ErrorObjectOwned>;

    #[method(name = "sendRawTransaction")]
    async fn send_raw_transaction(&self, raw_transaction: String) -> Result<(), ErrorObjectOwned>;

    #[method(name = "checkHealth")]
    async fn check_health(&self) -> Result<types::HealthCheckResponse, ErrorObjectOwned>;

    #[method(name = "requestCertificate")]
    async fn request_certificate(
        &self,
        object_id: [u8; 32],
        claim: Vec<u8>,
        // requested_at_block_height: u64
        // TODO: https://linear.app/quible/issue/QUI-106/generate-expiration-dates-corresponding-to-request-block-numbers
    ) -> Result<cert::types::SignedCertificate, ErrorObjectOwned>;

    #[method(name = "fetchUnspentValueOutputsByOwner")]
    async fn fetch_unspent_value_outputs_by_owner(
        &self,
        owner_address: [u8; 20],
    ) -> Result<ValueOutputsPayload, ErrorObjectOwned>;

    #[method(name = "requestFaucetOutput")]
    async fn request_faucet_output(&self) -> Result<FaucetOutputPayload, ErrorObjectOwned>;

    #[method(name = "getUnspentObjectOutputsByObjectId")]
    async fn get_unspent_object_outputs_by_object_id(
        &self,
        object_id: [u8; 32],
    ) -> Result<OutpointsPayload, ErrorObjectOwned>;

    #[method(name = "getClaimsByObjectId")]
    async fn get_claims_by_object_id(
        &self,
        object_id: [u8; 32],
    ) -> Result<ClaimsPayload, ErrorObjectOwned>;

    #[method(name = "getBlockHeight")]
    async fn get_block_height(&self) -> Result<BlockHeightPayload, ErrorObjectOwned>;

    #[method(name = "getBlockByHeight")]
    async fn get_block_by_height(
        &self,
        height_payload: BlockHeightPayload,
    ) -> Result<BlockDetailsPayload, ErrorObjectOwned>;
}
