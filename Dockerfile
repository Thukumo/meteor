# Build React web client
FROM node:slim AS client_builder
WORKDIR /app/client
COPY client/package*.json ./
RUN npm install --no-audit --no-fund
COPY client/ ./
RUN npm run lint && npm run build

# Build Rust server
FROM rust:slim AS server_builder
WORKDIR /app/server

# 依存しているクレートを事前にビルドしておく(キャッシュ)
COPY server/Cargo.toml server/Cargo.lock ./
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release

# touchすることで、上で置いたmain.rsよりも新しいタイムスタンプにする
COPY server/src ./src
RUN find src -type f -exec touch {} + && \
    cargo build --release

# Final image
FROM gcr.io/distroless/cc
WORKDIR /app
COPY --from=server_builder /app/server/target/release/server server
COPY --from=client_builder /app/client/dist static
COPY stream/ static/stream
EXPOSE 8080
ENTRYPOINT ["./server"]
