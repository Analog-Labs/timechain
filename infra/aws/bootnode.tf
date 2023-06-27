resource "aws_key_pair" "deployer" {
  key_name   = "${terraform.workspace}-terraform"
  public_key = var.ssh_pub_key
}

resource "aws_instance" "boot_node" {
  ami                         = var.ubuntu_ami
  instance_type               = var.ec2.instance_type
  vpc_security_group_ids      = [aws_security_group.sec_group.id]
  subnet_id                   = aws_subnet.public_subnets[0].id
  key_name                    = aws_key_pair.deployer.id

  depends_on = [
    aws_db_instance.timechain_db
  ]

  root_block_device {
    delete_on_termination = true
    encrypted             = false
    volume_size           = var.ec2.volume_size
    volume_type           = var.ec2.volume_type
  }

  tags = {
    Name = "${terraform.workspace}-bootnode"
    Role = "bootnode"
    Environment = "${terraform.workspace}"
  }

  user_data = <<EOF
#!/bin/bash
sudo apt update -y
sudo apt-get install -y apt-transport-https ca-certificates curl software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
sudo add-apt-repository -y "deb [arch=amd64] https://download.docker.com/linux/ubuntu  $(lsb_release -cs)  stable"
sudo apt update -y
sudo apt-get install -y docker-ce
sudo systemctl enable docker
sudo systemctl start docker
sudo groupadd docker
sudo usermod -aG docker $USER
docker pull ${var.timechain_image}

sudo docker run --name timenode \
    --restart always \
    -v /etc/timechain:/timechain \
    -e DATABASE_URL="postgresql://${aws_db_instance.timechain_db.address}:${aws_db_instance.timechain_db.port}/timechain?user=${var.db_user}&password=${var.db_password}" \
    -p 30333:30333 -p ${var.rpc_port}:${var.rpc_port} -p ${var.ws_port}:${var.ws_port} \
    -d ${var.timechain_image} \
      --validator \
      --base-path /timechain \
      --port 30333 --rpc-port=${var.rpc_port} \
      --chain local --alice --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
      --connector-url http://rosetta.analog.one:8081 \
      --connector-blockchain ethereum \
      --connector-network dev \
      --rpc-cors all \
      --rpc-external --rpc-methods unsafe --prometheus-external
EOF

}

# Public IP address for the bootnode
resource "aws_eip" "bootnode_ip" {
  tags = {
    Name = "${terraform.workspace}-bootnode-ip"
  }
}

# Associates the public IP address with the bootnode
resource "aws_eip_association" "bootnode_eip_association" {
  instance_id   = aws_instance.boot_node.id
  allocation_id = aws_eip.bootnode_ip.id
}

output "bootnode_private_ip" {
  value       = aws_instance.boot_node.private_ip
  description = "Bootnode Private IP"
}

output "bootnode_ip" {
  value       = aws_eip.bootnode_ip.public_ip
  description = "Bootnode"
}