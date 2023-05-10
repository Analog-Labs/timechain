resource "aws_key_pair" "deployer" {
  key_name   = "terraform"
  public_key = var.ssh_pub_key
}

resource "aws_instance" "boot_node" {
  ami                         = var.ubuntu_ami
  instance_type               = var.ec2.instance_type
  associate_public_ip_address = true
  vpc_security_group_ids      = [aws_security_group.testnet_sg.id]
  subnet_id                   = aws_subnet.public_subnets.id
  key_name                    = aws_key_pair.deployer.id

  root_block_device {
    delete_on_termination = true
    encrypted             = false
    volume_size           = var.ec2.volume_size
    volume_type           = var.ec2.volume_type
  }

  user_data = <<EOF
#!/bin/bash
sudo apt update -y
sudo apt-get install -y apt-transport-https ca-certificates curl software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
sudo add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu  $(lsb_release -cs)  stable"
sudo apt update -y
sudo apt-get install -y docker-ce
sudo systemctl start docker
sudo systemctl enable docker
sudo groupadd docker
sudo usermod -aG docker ubuntu
docker login ghcr.io -u sudachen -p ${var.ghcr_token}
docker pull ghcr.io/analog-labs/testnode
sudo docker run --name boot-node \
    -p 30333:30333 -p ${var.rpc_port}:${var.rpc_port} -p ${var.ws_port}:${var.ws_port} \
    -d ghcr.io/analog-labs/testnode \
      --validator \
      --base-path ./validator \
      --port 30333 --ws-port=${var.ws_port} --rpc-port=${var.rpc_port} \
      --chain local --alice --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
      --connector-url http://rosetta.analog.one:8081 \
      --connector-blockchain ethereum \
      --connector-network dev
EOF

}
