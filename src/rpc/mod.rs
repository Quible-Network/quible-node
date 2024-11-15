use jsonrpsee::proc_macros::rpc;
// use jsonrpsee::core::client::ClientT;
// use jsonrpsee::http_client::HttpClient;
// use jsonrpsee::rpc_params;
use jsonrpsee::types::ErrorObjectOwned;

use crate::tx::types::Transaction;
use crate::types;

#[rpc(server, client, namespace = "quible")]
pub trait QuibleRpc {
    // for some reason the macro makes RpcServerServer
    #[method(name = "sendTransaction")]
    async fn send_transaction(&self, transaction: Transaction) -> Result<(), ErrorObjectOwned>;

    #[method(name = "checkHealth")]
    async fn check_health(&self) -> Result<types::HealthCheckResponse, ErrorObjectOwned>;

    // TODO: https://linear.app/quible/issue/QUI-98/utxo-certificate-issuance
}
