variable "region" {}
variable "main_vpc_cidr" {}
variable "public_subnets" {}
variable "private_subnets" {}

variable "rpc_port" {}
variable "ws_port" {}
variable "boot_node_key" {}
variable "validators" {}

variable "ssh_pub_key" {}
variable "ghcr_token" {}

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

