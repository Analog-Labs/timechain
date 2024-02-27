#!/bin/bash
mkdir -p /etc/contracts

cp -r /temp_contracts/* /etc/contracts/
cp -r /temp_lib/* /etc/contracts/

exec /tester "$@"