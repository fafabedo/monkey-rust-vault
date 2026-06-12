FROM rust:1.88-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Cache dependency compilation layer.
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -r src
COPY src ./src
# Touch main.rs so cargo rebuilds it even if the dummy didn't change it.
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/monkey-vault /usr/local/bin/monkey-vault
EXPOSE 4200
CMD ["monkey-vault"]
