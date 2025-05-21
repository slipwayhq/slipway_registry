FROM rust:1.86 AS builder
WORKDIR /usr
COPY ./src ./src
COPY ./Cargo.* .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    update-ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/target/release/slipway_registry /usr/local/bin/slipway_registry

CMD ["slipway_registry"]
