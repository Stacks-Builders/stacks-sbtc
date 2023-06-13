#!/bin/bash
curl --data-binary '{"jsonrpc": "1.0", "id": "curltest", "method": "unloadwallet", "params": ["testdescriptorwallet"]}' -H 'content-type: text/plain;' http://abcd:abcd@127.0.0.1:18445/
