# Development

Use this command to run the node locally (with no persistence):

    cargo run --features surrealdb/kv-mem --bin quible-node

# Building a binary

In order to build a binary on MacOS, follow the steps from the [messense/homebrew-macos-cross-toolchains](https://github.com/messense/homebrew-macos-cross-toolchains/) repo, as seen below. You will need the `x86_64-unknown-linux-gnu` toolchain.

    brew tap messense/macos-cross-toolchains
    brew install x86_64-unknown-linux-gnu

Use this command to build the binary:

    cargo build --features surrealdb/protocol-ws --release --target=x86_64-unknown-linux-gnu

# Using docker-compose

Once you have built a binary, you can quickly get the Ubuntu-like environment running (without installing SurrealDB) by using Docker Compose.

Use this command to get started:

    docker-compose up

# Using the docker image

The docker image is intended for replicating an Ubuntu-like environment for development.

First, follow the instructions in _Building a binary_ to build a linux-x86_64 binary, which is required for building the docker image.

Also, install and run SurrealDB. You can follow the instructions [here](https://surrealdb.com/docs/surrealdb/installation/macos).

Use this command to start SurrealDB:

    surreal start

Use this command to build the image:

    docker build -t quible-node .

Use this command to run the image:

    docker run -p 9013:9013 --add-host=host.docker.internal:host-gateway -e QUIBLE_DATABASE_URL=ws://host.docker.internal:8000 --platform linux/x86_64 -it quible-node

# Deployment

#### Prerequisites

1. You must have Docker installed and running on your machine.
2. [INSERT SYSTEM REQUIREMENTS HERE]

#### Instructions (Linux x86_64)

1. `rustup target add x86_64-unknown-linux-gnu`
2. `cargo build --release --target x86_64-unknown-linux-gnu`

##### Instructions (MacOS)

1. `rustup target add x86_64-apple-darwin`
2. `cargo build --release --target x86_64-apple-darwin`

#### Instructions (Windows)

1. `rustup target add x86_64-pc-windows-msvc`
2. `cargo build --release --target x86_64-pc-windows-msvc`
