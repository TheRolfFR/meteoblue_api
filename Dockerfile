####################
#    BUILD RUST    #
####################
FROM clux/muslrust:1.83.0-stable AS builder

# create a new empty shell project
RUN USER=root cargo new --bin meteoblue_api

WORKDIR /app

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Copy and build
COPY ./src ./src
RUN cargo build --release

####################
#    FINAL IMAGE   #
####################
FROM alpine:latest

WORKDIR /root/

COPY --from=builder /app/target/*/release/meteoblue_api .

ENTRYPOINT ["./meteoblue_api"]
