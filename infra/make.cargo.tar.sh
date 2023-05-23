#!/usr/bin/env bash
cd `dirname $(realpath $0)`/.. && find . '(' -name Cargo.toml -o -name lib.rs -o -name build.rs -o -name main.rs ')' -exec tar -rvf cargo.toml.tar {} \;
