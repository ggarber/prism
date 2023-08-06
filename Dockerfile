FROM rust:1.66 as builder
WORKDIR /usr/src/prism
COPY . .
RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && rm -rf /var/lib/apt/lists/*
WORKDIR /prism
COPY --from=builder /usr/src/prism/target/release/prism /prism/prism
COPY --from=builder /usr/src/prism/demo /prism/demo
CMD ["prism"]
