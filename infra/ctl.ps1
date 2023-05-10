#!/usr/bin/env pwsh
Param([switch]$Up,[switch]$Down,[switch]$Build,[switch]$Upload,[switch]$Cargo)
Push-Location $PSScriptRoot

if ( $Cargo -or -not(Test-path ../cargo.toml.tar -PathType leaf)) {
    ./make.cargo.tar.sh
}

if ( $Build -or $Upload ) {
    docker build .. -f ./Dockerfile -t ghcr.io/analog-labs/testnode
}

if ( $Upload ) {
    docker push ghcr.io/analog-labs/timegraph-backend
}

if ( $Down -or $Up ) {
    docker compose down -v
}

if ( $Up ) {
    docker compose up -V --force-recreate -d
}

Pop-Location
