# Development

Use this command to run the node locally (with no persistence):

    make leader

Use this command to simulate other non-leader node peers:

    make follower

# Building a binary

In order to build a binary on MacOS, follow the steps from the [messense/homebrew-macos-cross-toolchains](https://github.com/messense/homebrew-macos-cross-toolchains/) repo, as seen below. You will need the `x86_64-unknown-linux-gnu` toolchain.

    brew tap messense/macos-cross-toolchains
    brew install x86_64-unknown-linux-gnu

Use this command to build the binary:

    make build

## Building a debian package

Build the debian package with this command:

    make build-deb

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
