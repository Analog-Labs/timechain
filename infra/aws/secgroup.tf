resource "aws_security_group" "sec_group" {
  vpc_id      = aws_vpc.internal_vpc.id
  dynamic "ingress" {
    for_each = var.security_groups
    content {
      from_port   = ingress.value["port"]
      description = ingress.value["name"]
      to_port     = ingress.value["port"]
      protocol    = ingress.value["protocol"]
      cidr_blocks      = ["0.0.0.0/0"]
      ipv6_cidr_blocks = ["::/0"]
    }
  }
  egress {
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }
}