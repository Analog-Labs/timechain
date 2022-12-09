#!/bin/bash

echo "Setting Bootnode Keys"
curl http://localhost:9944 -H "Content-Type:application/json;charset=utf-8" -d "@./session-keys/time1"
echo "Setting Validator01 Keys"
curl http://localhost:9946 -H "Content-Type:application/json;charset=utf-8" -d "@./session-keys/time2"
echo "Setting Validator02 Keys"
curl http://localhost:9948 -H "Content-Type:application/json;charset=utf-8" -d "@./session-keys/time3"
