resource "aws_iam_role" "lambda_role" {
    name = "lambda-role"

    assume_role_policy = <<EOF
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Action": "sts:AssumeRole",
            "Principal": {
                "Service": "lambda.amazonaws.com"
            },
            "Effect": "Allow",
            "Sid": ""
        }
    ]
}
EOF
}

resource "aws_iam_policy" "lambda_policy" {
    name = "lambda-policy"
    description = "Policy to allow Lambda to connect to VPC and access other resources."

    policy = <<EOF
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "ec2:CreateNetworkInterface",
                "ec2:DescribeNetworkInterfaces",
                "ec2:DeleteNetworkInterface",
                "secretsmanager:GetSecretValue",
                "secretsmanager:DescribeSecret",
                "secretsmanager:ListSecretVersionIds"
            ],
            "Resource": "*"
        }
    ]
}
EOF
}


resource "aws_iam_role_policy_attachment" "lambda_attachment" {
    role = aws_iam_role.lambda_role.name
    policy_arn = aws_iam_policy.lambda_policy.arn
}

resource "aws_lambda_function" "ff14_combat_log_lambda" {
    function_name = "ff14-combat-log-lambda"
    role = aws_iam_role.lambda_role.arn

    filename = "../../aws/lambda/ff14.zip"
    source_code_hash = filebase64sha256("../../aws/lambda/ff14.zip")

    handler = "not.used"
    memory_size = 128
    package_type = "Zip"
    reserved_concurrent_executions = 64
    runtime = "provided.al2"
    timeout = 30

    tags = {
        "lambda" = "ff14"
    }

    environment {
        variables = {
            "SQUADOV_LAMBDA_REGION" = "us-east-2"
            "SQUADOV_LAMBDA_DB_SECRET" = var.db_secret
            "SQUADOV_LAMBDA_DBHOST" = var.db_host
        }
    }

    vpc_config {
        subnet_ids = var.lambda_subnets
        security_group_ids = var.lambda_security_groups
    }
}