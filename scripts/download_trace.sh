set -x
blk=5224657
curl -s -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"scroll_getBlockTraceByNumberOrHash", "params": ["'$(printf '0x%x' $blk)'"], "id": 99}' 127.0.0.1:8545 > testdata/mainnet_blocks/${blk}.json 
