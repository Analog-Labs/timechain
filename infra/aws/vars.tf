variable "region" {}
variable "main_vpc_cidr" {}
variable "public_subnets" {}
variable "private_subnets" {}

variable "rpc_port" {}
variable "ws_port" {}
variable "boot_node_key" {}
variable "validators" {
  description = "List of validators to spin up"
  type = list(object({
    name = string
  }))
}

variable "ssh_pub_key" {}

variable "ubuntu_ami" {
  description = "ubuntu (ami us-east-1) Jammy 22.04 LTS amd64 hvm:ebs-ssd 20230428"
  type        = string
  default     = "ami-0044130ca185d0880"
}

variable "security_groups" {
  description = "The attribute of security_groups information"
  type = list(object({
    name        = string
    port        = number
    protocol    = string
  }))
}

variable "ec2" {
  description = "The attribute of EC2 information"
  type = object({
    name              = string
    instance_type     = string
    volume_size       = number
    volume_type       = string
  })
}

variable "zone_id" {
  description = "The Route53 hosted zone where to add the DNS records"
  type = string
}

variable "bootnode_dns" {
  description = "DNS A record for the bootnode"
  type = string
}

variable "timechain_image" {
  description = "Docker image to be used for the Timechain node"
  type = string
}

variable "db_size" {
  description = "Storage size for DB"
  type = number
}

variable "db_instance" {
  description = "RDS instance to use for the DB"
  type = string
  default = "db.m5d.large"  # Lower posible instance for Postgres
}

variable "db_user" {
  description = "Username for accessing the DB"
  type = string
}

variable "db_password" {
  description = "Password for accessing DB"
  type = string
}