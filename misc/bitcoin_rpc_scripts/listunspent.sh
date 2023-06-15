#!/bin/bash
curl --data-binary '{"jsonrpc": "1.0", "id": "curltest", "method": "listunspent", "params": [6, 9999999, ["bc1qyzxdu4px4jy8gwhcj82zpv7qzhvc0fvumgnh0r"]]}' -H 'content-type: text/plain;' http://abcd:abcd@127.0.0.1:18445/
