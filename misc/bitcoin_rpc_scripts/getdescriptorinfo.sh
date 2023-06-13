#!/bin/bash
curl --data-binary '{"jsonrpc": "1.0", "id": "curltest", "method": "getdescriptorinfo", "params": ["wpkh(020083b438ba636d30c3da6bbd73e6279438d4f987b6ebac9b32893f8e550f78db)"]}' -H 'content-type: text/plain;' http://abcd:abcd@127.0.0.1:18445/
