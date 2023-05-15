#!/bin/sh
mkdir -p .test
mkdir -p .coverage
clarinet run --allow-write ext/generate-tests.ts
rm -fR contracts-backup
cp -R contracts contracts-backup
rm -fR contracts
docker run -v `pwd`:/home ghcr.io/prompteco/clariform --format=spread --output-dir "contracts" contracts-backup/*.clar
mkdir contracts
cp -R contracts-spread/contracts-backup/* contracts
clarinet test --coverage .coverage/lcov.info .test
rm -fR contracts
cp -R contracts-backup contracts
