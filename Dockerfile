# Build Flutter web client
FROM dart:stable AS flutter_builder
WORKDIR /app/client
RUN useradd -m flutteruser
COPY client/ ./
RUN chown -R flutteruser:flutteruser /app/client
USER flutteruser
RUN apt update && apt install -y unzip xz-utils git curl && \
    git clone https://github.com/flutter/flutter.git -b stable /flutter && \
    export PATH="$PATH:/flutter/bin" && \
    /flutter/bin/flutter pub get && \
    /flutter/bin/flutter build web

# Build Rust server
FROM rust:slim-bookworm AS rust_builder
WORKDIR /app/server
COPY server/ ./
RUN cargo build --release

# Final image
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=rust_builder /app/server/target/release/server ./server
COPY --from=flutter_builder /app/client/build/web ./static
COPY stream/ ./static/stream
EXPOSE 8080
CMD ["./server"]
