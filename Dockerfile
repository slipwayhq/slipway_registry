FROM rust:1.86 AS builder
WORKDIR /usr
COPY ./src ./src
COPY ./Cargo.* .
RUN cargo build --release

FROM debian:bookworm-slim AS fetcher
RUN apt-get update && \
    apt-get install -y wget unzip && \
    wget https://github.com/grafana/alloy/releases/latest/download/alloy-linux-amd64.zip -O /alloy.zip && \
    unzip /alloy.zip -d /tmp && \
    mv /tmp/alloy-linux-amd64 /alloy && \
    chmod +x /alloy

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    update-ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/target/release/slipway_registry /usr/local/bin/slipway_registry
COPY --from=fetcher /alloy /usr/local/bin/alloy
COPY alloy.river /etc/alloy/alloy.river
COPY start.sh /usr/local/bin/start.sh
RUN chmod +x /usr/local/bin/start.sh

CMD ["/usr/local/bin/start.sh"]
