resource "aws_db_instance" "timechain_db" {
  identifier            = "${terraform.workspace}-database"
  engine                = "postgres"
  instance_class        = var.db_instance
  allocated_storage     = var.db_size 
  username              = var.db_user
  db_name               = "timechain" 
  password              = var.db_password
  publicly_accessible   = true
  skip_final_snapshot   = true # TODO: For production scenario, set this to false and keep the snapshot

  tags = {
    Name = "${terraform.workspace}-database"
  }
}

output "database_address" {
    value = aws_db_instance.timechain_db.address
}

output "database_port" {
    value = aws_db_instance.timechain_db.port
}

output "database_endpoint" {
  value = "${aws_db_instance.timechain_db.endpoint}"
}