# meteoblue_api

*Rust fast cli to get json from meteoblue city endpoint*

## Why?

**For educational purposes only.**

By extracting the data from URL, it can be used with any language or system understanding JSON.

## Usage

```
meteoblue_api_linux_musl <METEOBLUE_CITY_URL>
```

## Requirements

- Rust 1.95+

## How to run in dev

To run with html example file:
```
cargo run
```

To run with real URL:
```
cargo run -- https://www.meteoblue.com/en/weather/week/paris_france_2988507
```

## How to test with window opened

Just use `HEADLESS` env variable set to `false`:
```
LOGIN=toto PASSWORD=tata HEADLESS=false cargo run --bin meteoblue_api
```

## How to build release

First, clone this repo (duh).

Build steps:
1. Build the container
1. Create the container (but don't run it)
1. Copy the binaries out

```sh
sh build_and_extract_binaries.sh
```

## Run with different level of log

The cli utilizes ``env_logger`` to adjust level of debug. You can then choose which module has which log level [[Reference](https://docs.rs/env_logger/0.11.8/env_logger/#enabling-logging)]:
```
RUST_LOG=debug,playwright=off meteoblue_graph https://www.meteoblue.com/fr/meteo/semaine/paris_france_2988507
```

```
LOGIN=toto PASSWORD=tata TIMEOUT=2.0 IFACE=0.0.0.0 RUST_LOG=debug,playwright=off,iny_http=off,html5ever=off,selectors=off meteoblue_api
```
