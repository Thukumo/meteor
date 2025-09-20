# Build React web client
FROM node:slim AS client_builder
WORKDIR /app/client
COPY client/ ./
RUN npm install && npm run build

# Build Rust server
FROM rust:slim AS rust_builder
WORKDIR /app/server
COPY server/ ./
RUN cargo build --release

# Final image
FROM gcr.io/distroless/cc
WORKDIR /app
COPY --from=rust_builder /app/server/target/release/server server
COPY --from=client_builder /app/client/dist static
COPY stream/ static/stream
EXPOSE 8080
ENTRYPOINT ["./server"]
