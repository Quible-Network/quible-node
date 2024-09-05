# Use the official Rust image as a parent image
FROM rust:1.80.1

# Install system dependencies
RUN apt-get update && apt-get install -y \
    clang \
    libclang-dev \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory in the container
WORKDIR /usr/src/app

# Copy the entire project
COPY . .

# Build the application
RUN cargo build --release

# Specify the command to run the application
CMD ["cargo", "run", "--bin", "quible-node"]