# Build Flutter web client
FROM dart:stable AS flutter_builder
WORKDIR /app/client
COPY client/ ./
RUN apt-get update && apt-get install -y unzip xz-utils git curl && \
    git clone https://github.com/flutter/flutter.git -b stable /flutter && \
    export PATH="$PATH:/flutter/bin" && \
    /flutter/bin/flutter pub get && \
    /flutter/bin/flutter build web

# Build Rust server
FROM rust:1.77 as rust_builder
WORKDIR /app/server
COPY server/ ./
RUN cargo build --release

# Final image
FROM debian:bullseye-slim
WORKDIR /app
COPY --from=rust_builder /app/server/target/release/server ./server
COPY --from=flutter_builder /app/client/build/web ./static
COPY stream/ ./static/stream
EXPOSE 80
CMD ["./server"]
