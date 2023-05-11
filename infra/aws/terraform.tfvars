boot_node_key = "12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp"
validators = ["bob","charlie"]

ssh_pub_key = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC6kARZtSUd2T5bDX/9igFV9NhZAeGL/251ZXZJJhqtVfh8LKkIXCbe5hzgCK+qmTFhgdYt5OwMl7geAEbOSJ1AxjzlfaU/UBaw016iF0utqB9D/vCUebDhOkkvAvTBUzIU2DpWDRF1SN3CV06sIWlMlqSo7o70IwevW8wFdDcBePGVgXi51Yb5qWK4DpyAx2aMjE2qiKFyPwQRgstNtEqBv3kjOFjh+8cJ72sNQ7VahZJU+wxGUW4rhUjzlNQFY3YeXEdw+HurqmBcXswe6G04J1IAr4O1gn1eIYR7MhsF6h3eOvIIk8Oa4JVVqglk7YERpa/BJddJyaVZOve7ROINw8+LffwpURSR3DmXE3hTr0NG/gdBc5qbqv8dDiM0and8fRFe0XcsAtxxqQjf3cafUSc7is0NgI9IN2aB4QNeZ6a1S+qq1ukZ+UOqW3WoSud4S0LsAbT7mmHIERc120xGkYgy2SyYoAMvvRr46HgqDQduHJQ1juHUER6fNdeHb2U= alexe@Beast"
ghcr_token = "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"

region = "us-east-1"
main_vpc_cidr = "10.0.0.0/24"
public_subnets = "10.0.0.128/26"
private_subnets = "10.0.0.192/26"

ec2 = {
  instance_type     = "t3a.small"
  name              = "node"
  volume_size       = 20
  volume_type       = "gp3"
}

security_groups = [{
  name        = "SSH"
  protocol    = "tcp"
  port     = 22
  cidr_blocks = ["0.0.0.0/0"]
}, {
  name        = "substrate P2P/TCP"
  protocol    = "tcp"
  port        = 30333
  cidr_blocks = ["0.0.0.0/0"]
}, {
  name        = "substrate P2P/UDP"
  protocol    = "udp"
  port        = 30333
  cidr_blocks = ["0.0.0.0/0"]
}, {
  name        = "substrate RPC"
  protocol    = "tcp"
  port        = 9933
  cidr_blocks = ["0.0.0.0/0"]
}, {
  name        = "substrate WS"
  protocol    = "tcp"
  port        = 9944
  cidr_blocks = ["0.0.0.0/0"]
}]

# do not forget modify security group!
rpc_port = 9933
ws_port = 9944

