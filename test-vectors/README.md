
# About

Generates Test-vectors for various kinds of SBTC transactions.

Each of the individual files has a public function for generating a test vector that returns a `bitcoin::Transaction` data type.
This can then be serialized into a hex-string using the `serialize_tx` public function defined in utils