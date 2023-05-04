#!/usr/bin/env bash

apt-get update
apt-get install -y unzip curl lcov

curl -L -o clarinet 'https://drive.google.com/uc?export=download&confirm=yes&id=1amZ-VC53P8A2NAnwQ2bcduNBrzC-EzG7'
chmod +x ./clarinet
mv ./clarinet /usr/local/bin
