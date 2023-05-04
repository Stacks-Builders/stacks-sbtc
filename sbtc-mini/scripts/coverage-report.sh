#!/bin/sh
genhtml coverage.lcov -o .coverage/
open .coverage/index.html
