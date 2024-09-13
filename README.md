# Deployment

#### Prerequisites (MacOS)

```
brew tap messense/macos-cross-toolchains
# install x86_64-unknown-linux-gnu toolchain
brew install x86_64-unknown-linux-gnu
```

```
# build for linux x86_64
RUSTFLAGS="-C link-args=-fstack-protector-all -lssp" CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc cargo build --release --target x86_64-unknown-linux-gnu
```

#### Instructions (Linux x86_64)

1. `rustup target add x86_64-unknown-linux-gnu`
2. `cargo build --release --target x86_64-unknown-linux-gnu`

##### Instructions (MacOS)

1. `rustup target add x86_64-apple-darwin`
2. `cargo build --release --target x86_64-apple-darwin`

#### Instructions (Windows)

1. `rustup target add x86_64-pc-windows-msvc`
2. `cargo build --release --target x86_64-pc-windows-msvc`
