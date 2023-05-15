
data "aws_availability_zones" "available" {
  state = "available"
}

resource "aws_vpc" "testnet" {
  cidr_block       = var.main_vpc_cidr
  instance_tenancy = "default"
  enable_dns_hostnames = true
  enable_dns_support = true
}

resource "aws_internet_gateway" "testnet_igw" {
  vpc_id =  aws_vpc.testnet.id
}

resource "aws_subnet" "public_subnets" {
  vpc_id =  aws_vpc.testnet.id
  cidr_block = var.public_subnets
  availability_zone = data.aws_availability_zones.available.names[0]
  map_public_ip_on_launch = true
}

resource "aws_route_table" "public_rt" {
  vpc_id =  aws_vpc.testnet.id
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.testnet_igw.id
  }
}

resource "aws_route_table_association" "public_rt_assoc" {
  subnet_id = aws_subnet.public_subnets.id
  route_table_id = aws_route_table.public_rt.id
}


