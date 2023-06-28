boot_node_key = "12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp"
validators = [
  {
    name = "bob"
  },
  {
    name = "charlie"
  }
]

ssh_pub_key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJAFxp3e/C/FmWOzqHNADHkRvwIesNcGL487O20A/gKS glbranimir@gmail.com"

region = "us-east-1"
main_vpc_cidr = "10.0.0.0/24"
public_subnets = ["10.0.0.0/26", "10.0.0.128/26", "10.0.0.192/26"]

private_subnets = "10.0.0.192/26"

ec2 = {
  instance_type     = "t3a.small"
  name              = "node"
  volume_size       = 40
  volume_type       = "gp3"
}

security_groups = [
  {
    name        = "SSH"
    protocol    = "tcp"
    port        = 22
  }, 
  {
    name        = "substrate P2P/TCP"
    protocol    = "tcp"
    port        = 30333
  },
  {
    name        = "substrate RPC"
    protocol    = "tcp"
    port        = 9933
  },
  {
    name        = "substrate WS"
    protocol    = "tcp"
    port        = 9944
  },
  {
    name        = "substrate P2P/TCP"
    protocol    = "udp"
    port        = 30333
  }
]

# do not forget modify security group!
rpc_port = 9933
ws_port = 9944

timechain_image = "analoglabs/timechain:405fedc2"

zone_id = "Z056701228IF4U27MQKST"

bootnode_dns = "bootnode.internal.analog.one"

db_size = 80
db_user = "analog_user"
db_password = "Kl1mNsAj309A12945i"
