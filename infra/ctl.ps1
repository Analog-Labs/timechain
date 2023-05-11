#!/usr/bin/env pwsh
Param(
    [switch]$Up,
    [switch]$Down,
    [switch]$Build,
    [switch]$Upload,
    [switch]$Cargo,
    [switch]$Restart)

Push-Location $PSScriptRoot

if ($Cargo -or -not(Test-path ../cargo.toml.tar -PathType leaf))
{
    ./make.cargo.tar.sh
}

if ($Build -or $Upload)
{
    docker build .. -f ./Dockerfile -t ghcr.io/analog-labs/testnet
}

if ($Upload)
{
    docker push ghcr.io/analog-labs/testnet
}

if ($Down -or $Up)
{
    docker compose down -v
}

if ($Up)
{
    docker compose up -V --force-recreate -d
}

if ($Restart)
{
    Push-Location aws
    terraform apply -destroy -target aws_instance.validator_node -target aws_instance.boot_node -auto-approve
    terraform apply -auto-approve
    Pop-Location
}

Pop-Location
