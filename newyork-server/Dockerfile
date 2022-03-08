FROM rust:latest as builder

WORKDIR /app
COPY . .

RUN rustup default nightly
RUN apt-get clean
RUN apt-get update

RUN apt-get install -y libclang-dev
RUN cargo build --release

FROM ubuntu:latest
RUN apt-get update && apt-get install -y libclang-dev && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release /app

CMD ["./app/server_exec"]
