#!/bin/bash

set -ex

if [ $# -eq 0 ]; then
    echo "Usage: $0 <block_number> [url] [output_dir]"
    exit 0
fi

BLOCK=$1
URL=${2:-http://localhost:8545}
OUT_DIR=${3:-testdata/holesky_witness}

HEX_BLOCK=$(printf '0x%x' $BLOCK)

mkdir -p ${OUT_DIR}/${HEX_BLOCK}

cast rpc -r $URL eth_getBlockByNumber "$HEX_BLOCK" true | jq > ${OUT_DIR}/${HEX_BLOCK}/block.json
cast rpc -r $URL debug_executionWitness "$HEX_BLOCK" > ${OUT_DIR}/${HEX_BLOCK}/witness.json
