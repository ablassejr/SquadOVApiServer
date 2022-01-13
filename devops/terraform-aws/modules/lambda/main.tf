resource "aws_vpc" "lambda" {
    cidr_block = "192.168.0.0/16"
    instance_tenancy = "default"
}

resource "aws_internet_gateway" "lambda_gateway" {
    vpc_id = aws_vpc.lambda.id
}

resource "aws_vpc_peering_connection" "lambda_primary" {
    peer_vpc_id = var.primary_vpc
    vpc_id = aws_vpc.lambda.id
    auto_accept = true
}

resource "aws_subnet" "lambda_subnet_a" {
    vpc_id = aws_vpc.lambda.id
    availability_zone = "us-east-2a"
    cidr_block = "192.168.0.0/17"
}

resource "aws_subnet" "lambda_subnet_c" {
    vpc_id = aws_vpc.lambda.id
    availability_zone = "us-east-2c"
    cidr_block = "192.168.128.0/17"
}


resource "aws_eip" "lambda_nat_eip_a" {
    vpc = true
}

resource "aws_nat_gateway" "lambda_nat_a" {
    allocation_id = aws_eip.lambda_nat_eip_a.id
    connectivity_type = "public"
    subnet_id = aws_subnet.lambda_subnet_a.id

    depends_on = [ aws_internet_gateway.lambda_gateway ]
}

resource "aws_eip" "lambda_nat_eip_c" {
    vpc = true
}

resource "aws_nat_gateway" "lambda_nat_c" {
    allocation_id = aws_eip.lambda_nat_eip_c.id
    connectivity_type = "public"
    subnet_id = aws_subnet.lambda_subnet_c.id

    depends_on = [ aws_internet_gateway.lambda_gateway ]
}

resource "aws_route_table" "lambda_route_table_a" {
    vpc_id = aws_vpc.lambda.id

    route {
        cidr_block = "10.0.0.0/16"
        vpc_peering_connection_id = aws_vpc_peering_connection.lambda_primary.id
    }

    route {
        cidr_block = "0.0.0.0/0"
        nat_gateway_id = aws_nat_gateway.lambda_nat_a.id
    }
}

resource "aws_route_table_association" "lambda_rt_subnet_a" {
    route_table_id = aws_route_table.lambda_route_table_a.id
    subnet_id = aws_subnet.lambda_subnet_a.id
}


resource "aws_route_table" "lambda_route_table_c" {
    vpc_id = aws_vpc.lambda.id

    route {
        cidr_block = "10.0.0.0/16"
        vpc_peering_connection_id = aws_vpc_peering_connection.lambda_primary.id
    }

    route {
        cidr_block = "0.0.0.0/0"
        nat_gateway_id = aws_nat_gateway.lambda_nat_c.id
    }
}

resource "aws_route_table_association" "lambda_rt_subnet_c" {
    route_table_id = aws_route_table.lambda_route_table_c.id
    subnet_id = aws_subnet.lambda_subnet_c.id
}


resource "aws_security_group" "lambda_security_group" {
    name = "lambda-security-group"
    description = "Security group for the primary VPC for Lambda."
    vpc_id = aws_vpc.lambda.id

    ingress {
        from_port = 0
        to_port = 0
        protocol = "-1"
        cidr_blocks = ["0.0.0.0/0"]
    }

    egress {
        from_port = 0
        to_port = 0
        protocol = "-1"
        cidr_blocks = ["0.0.0.0/0"]
    }
}

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

resource "aws_iam_policy" "lambda_vpc_policy" {
    name = "lambda-policy-policy"
    description = "Policy to allow Lambda to connect to VPC."

    policy = <<EOF
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "ec2:CreateNetworkInterface",
                "ec2:DescribeNetworkInterfaces",
                "ec2:DeleteNetworkInterface"
            ],
            "Resource": "*"
        }
    ]
}
EOF
}

resource "aws_iam_role_policy_attachment" "lambda_vpc_attachment" {
    role = aws_iam_role.lambda_role.name
    policy_arn = aws_iam_policy.lambda_vpc_policy.arn
}

#resource "aws_lambda_function" "ff14_lambda" {
#    function_name = "ff14_combat_log"
#    role = aws_iam_role.lambda_role.arn
    
#    reserved_concurrent_executions = 64
#    runtime = "provided.al2"

#    timeout = 60

#    vpc_config {
#        subnet_ids = [aws_subnet.lambda_subnet_a.id, aws_subnet.lambda_subnet_c.id]
#        security_group_ids = [aws_security_group.lambda_security_group.id]
#    }
#}