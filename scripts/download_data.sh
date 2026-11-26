#!/bin/sh
set -e

cd "$(dirname "$0")/.."
mkdir -p maps

curl -fL -o maps/map_pool.bin \
    https://f004.backblazeb2.com/file/z3-map-rando-artifacts/map-pools/map-pool-20261121-070108-44913.bin
