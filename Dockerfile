# syntax=docker/dockerfile:1

FROM node:24-bookworm AS node

FROM rust:1.97.1-bookworm AS build
COPY --from=node /usr/local/ /usr/local/
RUN apt-get update && apt-get install -y --no-install-recommends clang cmake llvm && rm -r /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown && cargo install --locked wasm-pack --version 0.13.0

WORKDIR /src
COPY frontend/package.json frontend/package-lock.json ./frontend/
RUN npm --prefix frontend ci

COPY Cargo.toml Cargo.lock VERSION catalog-build.default.toml package.json ./
COPY crates/ crates/
COPY schema/ schema/
COPY data/ data/
COPY patches/ patches/
COPY asar/ asar/
COPY ALTTPRetiling/ ALTTPRetiling/
COPY z3-json-data/ z3-json-data/
COPY frontend/ frontend/
ADD https://f004.backblazeb2.com/file/z3-map-rando-artifacts/map-pools/map-pool-20261121-070108-44913.bin maps/map_pool.bin
RUN cargo build --locked --release --package server
RUN npm run build

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -r /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /src/target/release/z3-map-rando-server /usr/local/bin/
COPY --from=build /src/frontend/build/ frontend/build/
COPY --from=build /src/build/*_catalog.bin /src/build/version.txt build/
RUN mkdir maps
COPY --from=build --chmod=644 /src/maps/map_pool.bin maps/map_pool.bin

USER 65534:65534
EXPOSE 8080
ENTRYPOINT ["z3-map-rando-server", "--bind", "0.0.0.0:8080"]
