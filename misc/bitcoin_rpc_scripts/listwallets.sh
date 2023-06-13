#!/bin/bash
curl --data-binary '{"jsonrpc": "1.0", "id": "curltest", "method": "listwallets", "params": []}' -H 'content-type: text/plain;' http://abcd:abcd@127.0.0.1:18445/
