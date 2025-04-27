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

-Rust 1.83+

## How to run in dev

To run with html example file:
```
cargo run
```

To run with real URL:
```
cargo run -- https://www.meteoblue.com/en/weather/week/paris_france_2988507
```


## How to build release

First, clone this repo (duh).

Build steps:
1. Build the container
1. Create the container (but don't run it)
1. Copy the binary out

```
podman build -t meteoblue_api:dev .
podman rm extract_meteoblue
podman create --name extract_meteoblue meteoblue_api:dev
podman cp extract_meteoblue:/root/meteoblue_api ./meteoblue_api_linux_musl
```