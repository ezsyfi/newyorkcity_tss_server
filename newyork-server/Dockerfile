FROM rust:latest as builder

WORKDIR /app
COPY . .

RUN rustup default nightly
RUN apt-get clean
RUN apt-get update
RUN apt-get install -y libclang-dev
RUN cargo build --release

FROM ubuntu:latest

WORKDIR /app

# Install <extra-runtime-dependencies>
RUN apt-get update && apt-get install -y libclang-dev && rm -rf /var/lib/apt/lists/*

# Copy bin & config files
COPY --from=builder /app/target/release .
COPY --from=builder /app/Rocket.toml .

EXPOSE 8001

CMD ["./server_exec"]
