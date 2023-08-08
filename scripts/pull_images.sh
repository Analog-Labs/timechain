#!/bin/sh
docker image pull ethereum/client-go:v1.10.26
docker image pull staketechnologies/astar-collator:latest
docker image pull analoglabs/connector-ethereum:latest
docker image pull analoglabs/connector-astar:latest