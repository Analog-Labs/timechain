
data "aws_availability_zones" "available" {
  state = "available"
}

resource "aws_vpc" "internal_vpc" {
  cidr_block       = var.main_vpc_cidr
  instance_tenancy = "default"
  enable_dns_hostnames = true
  enable_dns_support = true

  tags = {
    Name = "${terraform.workspace}-vpc"
  }
}

resource "aws_internet_gateway" "igw" {
  vpc_id =  aws_vpc.internal_vpc.id
}

resource "aws_subnet" "public_subnets" {
  count                   = length(var.public_subnets)
  vpc_id                  = aws_vpc.internal_vpc.id
  cidr_block              = var.public_subnets[count.index]
  availability_zone       = data.aws_availability_zones.available.names[count.index]
  map_public_ip_on_launch = true

  tags = {
    Name = "${terraform.workspace}-subnet-${count.index+1}"
  }
}

resource "aws_route_table" "public_rt" {
  vpc_id =  aws_vpc.internal_vpc.id
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.igw.id
  }
}

resource "aws_route_table_association" "public_rt_assoc" {
  count           = length(aws_subnet.public_subnets)
  subnet_id       = aws_subnet.public_subnets[count.index].id
  route_table_id  = aws_route_table.public_rt.id
}


