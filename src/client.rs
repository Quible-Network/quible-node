use jsonrpsee::http_client::HttpClient;
mod quible_rpc;
mod types;

use quible_rpc::QuibleRpcClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = "http://localhost:9013";
    let client = HttpClient::builder().build(url)?;
    let transaction = types::Transaction {
        author: [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0,
        ],
        events: vec![types::Event::CreateQuirkle {
            members: vec![],
            proof_ttl: 86400,
        }],
    };

    // let params = rpc_params![transaction];
    // let params = rpc_params![json!({"events": [{"name": "CreateQuirkle", "members": [], "proof_ttl": 86400}]})];
    // let response: Result<String, _> = client.request("quible_sendTransaction", params).await;
    let response = client.send_transaction(transaction).await.unwrap();
    println!("{:?}", response);

    Ok(())
}
