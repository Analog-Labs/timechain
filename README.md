<div align="center">

  <h1><code>Timechain Node</code></h1>

  <strong>This repo is for the timechain node, which is part of the analog network. <a href="https://github.com/analog-labs">Analog</a>.</strong>

  <h3>
    <a href="https://analog.one/">Docs</a>
    <span> | </span>
    <a href="mailto:hello@analog.one">Support</a>
  </h3>

</div>

## Features

This repo includes the minimum required components to start a timechain testnet node, inspired by [substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template).

* Consensus related pallets: Aura & GRANDPA
* Token vesting pallet: analog-vesting store the vesting schedule and the vested balance of the account.
* Account delegation pallet: pallet-proxy store the delegation relationship between master account and delegator account. Delegate account can submit the task and pay task fee from master account.
* Task metadata storage pallet: pallet-task-metadata store the static configuration of the task.
* Task schedule pallet: pallet-task-schedule store the task schedule and its status. now it support the one-time task, repetitive task and payable task.
* Tesseract sig storage pallet: pallet-tesseract-sig-storage store the TSS shards and the signature collected from TSS workers.
* Task executor worker: run the tasks on chain.
* TSS worker: receive the data fetched from task executor, verify and sign against the data, and send the extrinsic to store the signature if reaches the threshold.

**Notes:** The code is still under active development and not production ready, use it at your own risk.

## Getting Started

Follow the steps below to get started.

1. Install docker, docker compose and musl using your system package manager.
2. Build the docker image with `./scripts/build_docker.sh`.
3. Start the timechain validators, chronicles and chain rpc nodes using `docker compose --profile evm up -d`.
4. Setup and send a gmp message from chain 2 to chain 3 `docker compose run --rm tc-cli --config=local-evm.yaml smoke-test 2 3`

below are some notes for specific OS'es:

### Debian/Ubuntu
if you're on debian/ubuntu system, install [musl-tools](https://packages.debian.org/sid/musl-tools) for enable musl-target cross-compilation:
```bash
apt-get install musl-tools
```

### MacOS
if you're on macos, install [musl-cross](https://github.com/FiloSottile/homebrew-musl-cross) for enable musl-target cross-compilation:
```bash
brew install filosottile/musl-cross/musl-cross --with-aarch64
```

add the following to your `.cargo/config`:
```toml
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"

[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-musl-gcc"

[env]
CC_x86_64-unknown-linux-musl = "x86_64-linux-musl-gcc"
CC_aarch64-unknown-linux-musl = "aarch64-linux-musl-gcc"
```

## Logging

We use a modified version of substrate that logs to JSON. Therefore we recommend running logs through the `json` filter in loki to more easily filter for all kinds of json entries.

For example to query the development eth chronicle for any none-info log messages you can use the following query.

```
{container="testnet-chronicle-eth-1"} | json | level != `INFO`
```

The `json` filter flattens all labels, so the json path `fields > log.target` is turned into the key `fields_log_target`, so keep that in mind when filtering.

Use the "Prettify JSON" option to improve readability in Grafana or the raw logcli output `--output raw` and a pipe to `jq` on the command line.
