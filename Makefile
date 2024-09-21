.PHONY: leader
leader:
	cargo run --bin quible-node -F surrealdb/kv-mem

.PHONY: follower
follower:
	QUIBLE_P2P_PORT=0 \
		QUIBLE_RPC_PORT=0 \
		QUIBLE_LEADER_MULTIADDR=/ip4/127.0.0.1/tcp/9014 \
		QUIBLE_SIGNER_KEY=$$(openssl rand -hex 32) \
		cargo run --bin quible-node -F surrealdb/kv-mem
