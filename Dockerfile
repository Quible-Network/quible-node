FROM ubuntu
RUN mkdir /app
WORKDIR /app
COPY ./target/x86_64-unknown-linux-gnu/release/quible-node /app/quible-node
RUN chmod +x /app/quible-node

ENTRYPOINT ["/app/quible-node"]
