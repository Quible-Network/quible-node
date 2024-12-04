CARGO_DEB_INSTALLED=$(shell cargo deb --version > /dev/null 2>&1; echo $$?)
UNAME=$(shell uname)

.PHONY: all
all:
	@echo "available commands:"
	@echo "  make leader"
	@echo "  make follower"
	@echo "  make example-app"
	@echo "  make build"
	@echo "  make build-deb"

.PHONY: leader
leader:
	QUIBLE_SIGNER_KEY=ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
	QUIBLE_DATABASE_URL=ws://localhost:8000 \
	cargo run \
		--features surrealdb/protocol-ws \
		--bin quible-node

.PHONY: follower
follower:
	QUIBLE_P2P_PORT=0 \
		QUIBLE_RPC_PORT=0 \
		QUIBLE_LEADER_MULTIADDR=/ip4/127.0.0.1/tcp/9014 \
		QUIBLE_SIGNER_KEY=$$(openssl rand -hex 32) \
		cargo run \
		--features surrealdb/kv-mem \
		--bin quible-node

.PHONY: example-app
example-app: sdk/node_modules
	cd sdk && npm run dev

sdk/node_modules: sdk/package.json $(wildcard sdk/apps/*/package.json) $(wildcard sdk/packages/*/package.json)
	cd sdk && npm install

.PHONY: build
build: target/x86_64-unknown-linux-gnu/release/quible-node

.PHONY: build-deb
build-deb: target/x86_64-unknown-linux-gnu/debian

.PHONY: cargo-deb
ifneq ($(CARGO_DEB_INSTALLED), 0)
cargo-deb:
	cargo install cargo-deb
endif

target/x86_64-unknown-linux-gnu/debian: target/x86_64-unknown-linux-gnu/release/quible-node cargo-deb
	cargo deb --no-build --target=x86_64-unknown-linux-gnu

ifeq ($(UNAME), Darwin)
target/x86_64-unknown-linux-gnu/release/quible-node: Cargo.toml Cargo.lock $(wildcard src/*.rs)
	cargo build \
		--config .cargo/macos-cross.toml \
		--features surrealdb/protocol-ws \
		--release \
		--target=x86_64-unknown-linux-gnu
else
target/x86_64-unknown-linux-gnu/release/quible-node: Cargo.toml Cargo.lock $(wildcard src/*.rs)
	cargo build \
		--features surrealdb/protocol-ws \
		--release \
		--target=x86_64-unknown-linux-gnu
endif
