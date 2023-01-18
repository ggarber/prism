FROM rust:1.66 as builder
WORKDIR /usr/src/prism
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/prism /usr/local/bin/prism
CMD ["prism"]

