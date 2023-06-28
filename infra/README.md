# Timechain infrastructure

## Intro

`infra` folder contains all terraform files for spinning up a Timechain network.
Currently only AWS is supported.

When applied, the configuration creates:
- 1 bootnode + N validator nodes (EC2 instances running the Analog Docker container)
- static IP addresses with a default security group
- N subnets specified in the `public_subnets` variable
- Load balancer pointing to the bootnode with a DNS record
- SSL certificates for RPC
- Postgres database

## Usage

To enable deployment on multiple environments (internal, qa, prod, test, etc.), this project uses Terraform workspace.
First be sure to have a folder under `./environments` with the name of the workspace (for example, testnet).

Be sure to have a workspace synced with the local lock file:

`terraform worskpace list`

To create a new workspace, create the environment folder with the variable file (`terraform.tfvars`) and add a Terraform workspace:

`terraform workspace new {ENV}`

Before creating the resources, be sure to select the right workspace:

`terraform workspace select {ENV}`

To apply te configuration, execute:

`terraform apply -var-file=environments/{ENV}/terraform.tfvars`

