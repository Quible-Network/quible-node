use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::rpc_params;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let url = "http://localhost:9013";
	let client = HttpClient::builder().build(url)?;
  let transaction = types::Transaction {
      events: vec![
          types::Event::CreateQuirkle {
              members: vec![],
              proof_ttl: 86400
          }
      ]
  };

	let params = rpc_params![transaction];
	// let params = rpc_params![json!({"events": [{"name": "CreateQuirkle", "members": [], "proof_ttl": 86400}]})];
	let response: Result<String, _> = client.request("quible_sendTransaction", params).await;
	println!("{:?}", response);

	Ok(())
}
