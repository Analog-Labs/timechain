resource "aws_instance" "validator_node" {
  count = length(var.validators)

  ami                         = var.ubuntu_ami
  instance_type               = var.ec2.instance_type
  associate_public_ip_address = true
  vpc_security_group_ids      = [aws_security_group.sec_group.id]
  subnet_id                   = aws_subnet.public_subnets[count.index].id
  key_name                    = aws_key_pair.deployer.id

  depends_on = [ 
    aws_db_instance.timechain_db,
    aws_eip.bootnode_ip
  ]

  root_block_device {
    delete_on_termination = true
    encrypted             = false
    volume_size           = var.ec2.volume_size
    volume_type           = var.ec2.volume_type
  }

  tags = {
    Name = "${terraform.workspace}-validator-${count.index+1}"
    Role = "validator"
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

sudo docker run -it \
  -e WAIT_HOSTS="${aws_eip.bootnode_ip.public_ip}:30333" \
  --entrypoint=/wait ${var.timechain_image}

sudo docker run --name validator-node \
  --restart always \
  -p 30333:30333 -p ${var.rpc_port}:${var.rpc_port} -p ${var.ws_port}:${var.ws_port} \
  -e DATABASE_URL="postgresql://${aws_db_instance.timechain_db.address}:${aws_db_instance.timechain_db.port}/timechain?user=${var.db_user}&password=${var.db_password}" \
  -d ${var.timechain_image} \
    --validator \
    --base-path /timechain \
    --port 30333 --rpc-port=${var.rpc_port} \
    --chain local --${var.validators[count.index].name} \
    --bootnodes /ip4/${aws_eip.bootnode_ip.public_ip}/tcp/30333/p2p/${var.boot_node_key} \
    --connector-url http://rosetta.analog.one:8081 \
    --connector-blockchain ethereum \
    --connector-network dev \
    --rpc-cors all \
    --rpc-external --rpc-methods unsafe --prometheus-external
EOF

}

output "validators_ip" {
  value = aws_instance.validator_node[*].public_ip
  description = "Validators"
}
