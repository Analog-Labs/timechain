resource "aws_iam_role" "ec2_role_testnet" {
  name = "ec2_role_testnet"

  assume_role_policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Action": "sts:AssumeRole",
      "Principal": {
        "Service": "ec2.amazonaws.com"
      },
      "Effect": "Allow"
    }
  ]
}
EOF

  tags = {
    project = "testnet"
  }
}


resource "aws_iam_instance_profile" "ec2_profile_testnet" {
  name = "ec2_profile_testnet"
  role = aws_iam_role.ec2_role_testnet.name
}

resource "aws_iam_role_policy" "ec2_policy_testnet" {
  name = "ec2_policy_testnet"
  role = aws_iam_role.ec2_role_testnet.id

  policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Action": [
        "ecr:GetAuthorizationToken",
        "ecr:BatchGetImage",
        "ecr:GetDownloadUrlForLayer"
      ],
      "Effect": "Allow",
      "Resource": "*"
    }
  ]
}
EOF
}
