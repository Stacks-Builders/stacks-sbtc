#!/bin/sh
mkdir -p .test
mkdir -p .coverage
clarinet run --allow-write ext/generate-tests.ts
mkdir -p contracts-backup
cp -R contracts/*.clar contracts-backup/
rm -fR contracts/*.clar
docker run -v $(pwd):/home ghcr.io/prompteco/clariform --format=spread --output-dir "contracts-spread" contracts-backup/*.clar
cp -R contracts-spread/contracts-backup/* contracts
clarinet test --coverage .coverage/lcov.info .test
sudo rm -fR contracts/*.clar
cp -R contracts-backup/*.clar contracts
sudo rm -fR contracts-backup
sudo rm -fR contracts-spread
