# Build Flutter web client
FROM ghcr.io/cirruslabs/flutter:latest AS flutter_builder
WORKDIR /app/client
COPY client/ ./
RUN flutter pub get && flutter build web --release

# Build Rust server
FROM rust:latest AS rust_builder
WORKDIR /app/server
COPY server/ ./
RUN cargo build --release

# Final image
FROM gcr.io/distroless/cc
WORKDIR /app
COPY --from=rust_builder /app/server/target/release/server server
COPY --from=flutter_builder /app/client/build/web static
COPY stream/ static/stream
EXPOSE 8080
ENTRYPOINT ["./server"]
