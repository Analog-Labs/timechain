#!/bin/bash

printf "Setting Bootnode TimeKey\n"
curl http://localhost:9944 -H "Content-Type:application/json;charset=utf-8" -d "@./session-keys/time1"
printf "\nSetting Validator01 TimeKey\n"
curl http://localhost:9946 -H "Content-Type:application/json;charset=utf-8" -d "@./session-keys/time2"
printf "\nSetting Validator02 TimeKey\n"
curl http://localhost:9948 -H "Content-Type:application/json;charset=utf-8" -d "@./session-keys/time3"
