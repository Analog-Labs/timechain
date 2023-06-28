# Application load balancer for RPC port
resource "aws_lb" "app_load_balancer" {
  name                = "${terraform.workspace}-load-balancer"
  load_balancer_type  = "application"
  subnets             = aws_subnet.public_subnets[*].id
  internal            = false
  security_groups     = [aws_security_group.sec_group.id]

  tags = {
    Name = "${terraform.workspace}-load-balancer"
  }

  lifecycle {
    create_before_destroy = true
  }
}

# Target group for the RPC port on the load balancer
resource "aws_lb_target_group" "timenode_target_group" {
  name        = "timenode-target-group"
  port        = var.rpc_port
  protocol    = "HTTP"
  vpc_id      = aws_vpc.internal_vpc.id
  target_type = "instance"

  health_check {
    path = "/health"
  }
}

# Binds the target group to the machine 
# NOTE: for now, only includes the bootnode
resource "aws_lb_target_group_attachment" "timenode_target_group_attachment" {
  count             = 1
  target_group_arn  = aws_lb_target_group.timenode_target_group.arn
  target_id         = aws_instance.boot_node.id
  port              = var.rpc_port
}

# Front listener for the LB on RPC port
resource "aws_lb_listener" "front_listener" {
  load_balancer_arn = aws_lb.app_load_balancer.arn
  port = var.rpc_port
  protocol = "HTTPS"
  certificate_arn = aws_acm_certificate.bootnode_cert.arn

  default_action {
    type = "forward"
    target_group_arn = aws_lb_target_group.timenode_target_group.arn
  }
}


resource "aws_route53_record" "bootnode_record" {
  zone_id         = var.zone_id
  name            = var.bootnode_dns
  type            = "CNAME"
  ttl             = 300
  records         = [aws_lb.app_load_balancer.dns_name]
  allow_overwrite = true
}

# Certificate for the bootnode DNS record
resource "aws_acm_certificate" "bootnode_cert" {
  domain_name       = var.bootnode_dns
  validation_method = "DNS"

  lifecycle {
    create_before_destroy = true
  }
}

# Certificate validation DNS record
resource "aws_route53_record" "bootnode_cert_dns" {
  allow_overwrite = true
  name =  tolist(aws_acm_certificate.bootnode_cert.domain_validation_options)[0].resource_record_name
  records = [tolist(aws_acm_certificate.bootnode_cert.domain_validation_options)[0].resource_record_value]
  type = tolist(aws_acm_certificate.bootnode_cert.domain_validation_options)[0].resource_record_type
  zone_id = var.zone_id
  ttl = 60
}

resource "aws_acm_certificate_validation" "bootnode_cert_validation" {
  certificate_arn         = aws_acm_certificate.bootnode_cert.arn
  validation_record_fqdns = [aws_route53_record.bootnode_cert_dns.fqdn]
}

resource "aws_lb_listener_certificate" "lb_cert" {
  listener_arn = aws_lb_listener.front_listener.arn
  certificate_arn = aws_acm_certificate.bootnode_cert.arn
}
