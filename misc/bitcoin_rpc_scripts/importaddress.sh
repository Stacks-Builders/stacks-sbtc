#!/bin/bash
curl --data-binary '{"jsonrpc": "1.0", "id": "curltest", "method": "importdescriptors", "params": [[{ "desc": "wpkh(020083b438ba636d30c3da6bbd73e6279438d4f987b6ebac9b32893f8e550f78db)#alj2qhtz", "timestamp": "now", "internal": false, "watchonly": true, "label": "", "keypool": true, "rescan": true }]]}' -H 'content-type: text/plain;' http://abcd:abcd@127.0.0.1:18445/wallet/testdescriptorwallet

# bitcoin-cli importdescriptors '[{ "desc": "<my descriptor>", "timestamp":1455191478, "active": true, "range": [0,100], "label": "<my bech32 wallet>" }]'
