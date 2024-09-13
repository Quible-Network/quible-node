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
