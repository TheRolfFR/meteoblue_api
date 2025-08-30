####################
#    BUILD RUST    #
####################
FROM clux/muslrust:1.83.0-stable AS builder

# Add openssl
RUN apt-get update && apt-get install -y linux-headers-generic musl-tools openssl pkg-config libssl-dev

# create a new empty shell project
RUN USER=root cargo new --vcs none --bin meteoblue_api

WORKDIR /app

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Copy and build
COPY ./src ./src
RUN cargo build --release --all-features

####################
#    FINAL IMAGE   #
####################
FROM alpine:latest
RUN apk --no-cache add ca-certificates

WORKDIR /root/

COPY --from=builder /app/target/*/release/meteoblue_* .

ENTRYPOINT ["./meteoblue_api", "--help"]
