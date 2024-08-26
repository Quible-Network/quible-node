use jsonrpsee::http_client::HttpClient;
use quible_ecdsa_utils::sign_message;
use quible_rpc::QuibleRpcClient;
use quible_transaction_utils::compute_transaction_hash;
use alloy_primitives::FixedBytes;

mod quible_rpc;
mod quible_ecdsa_utils;
mod quible_transaction_utils;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = "http://localhost:9013";
    let client = HttpClient::builder().build(url)?;

        let signer_secret: [u8; 32] = [
            0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0,
            0x0, 0x0, 0x0, 0x0,
        ];
        let events = vec![types::Event::CreateQuirkle {
                members: vec!["foo".to_string()],
                proof_ttl: 86400,
            }];
        let hash = compute_transaction_hash(&events);
        let signature_bytes = sign_message(FixedBytes::new(signer_secret), FixedBytes::new(hash))?;
        let signature = types::ECDSASignature { bytes: signature_bytes };
        let transaction = types::Transaction { signature, events };

    // let params = rpc_params![transaction];
    // let params = rpc_params![json!({"events": [{"name": "CreateQuirkle", "members": [], "proof_ttl": 86400}]})];
    // let response: Result<String, _> = client.request("quible_sendTransaction", params).await;
    let response = client.send_transaction(transaction).await.unwrap();
    println!("{:?}", response);

    Ok(())
}
