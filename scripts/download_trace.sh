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

PAYLOAD='{"jsonrpc":"2.0","method":"debug_executionWitness", "params": ["'$HEX_BLOCK'"], "id": 1}'
curl -H "Content-Type: application/json" -X POST --data "$PAYLOAD" $URL | jq .result > ${OUT_DIR}/${HEX_BLOCK}/witness.json

PAYLOAD='{"jsonrpc":"2.0","method":"eth_getBlockByNumber", "params": ["'$HEX_BLOCK'", true], "id": 1}'
curl -H "Content-Type: application/json" -X POST --data "$PAYLOAD" $URL | jq .result > ${OUT_DIR}/${HEX_BLOCK}/block.json