resource "aws_apigatewayv2_api" "combat_log_gateway" {
    name          = "combatlog-http-api"
    protocol_type = "HTTP"
}

resource "aws_iam_role" "combat_log_role" {
    name = "combat_log_role"
    assume_role_policy = <<POLICY
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Principal": {
                "Service": [
                    "apigateway.amazonaws.com"
                ]
            },
            "Action": "sts:AssumeRole"
        }
    ]
}
POLICY
}

resource "aws_iam_role_policy" "combat_log_policy" {
    name = "authenticated_cognito_policy"
    role = aws_iam_role.combat_log_role.id

    policy = <<EOF
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "kinesis:Put*"
            ],
            "Resource": [
                "*"
            ]
        }
    ]
}
EOF
}

resource "aws_apigatewayv2_stage" "combat_log_gateway_stage" {
    api_id = aws_apigatewayv2_api.combat_log_gateway.id
    auto_deploy = true
    name   = "$default"

    default_route_settings {
        detailed_metrics_enabled = true
        throttling_burst_limit = 5000
        throttling_rate_limit = 10000
    }

    access_log_settings {
        destination_arn = "arn:aws:logs:us-east-2:214663929182:log-group:aws-api-gateway"
        format = "{ \"requestId\":\"$context.requestId\", \"ip\": \"$context.identity.sourceIp\", \"requestTime\":\"$context.requestTime\", \"httpMethod\":\"$context.httpMethod\",\"routeKey\":\"$context.routeKey\", \"status\":\"$context.status\",\"protocol\":\"$context.protocol\", \"responseLength\":\"$context.responseLength\" }"
    }
}

output "api_gateway_id" {
    value = aws_apigatewayv2_api.combat_log_gateway.id
}

resource "aws_s3_bucket_notification" "combat_log_bucket_notification" {
    bucket = data.aws_s3_bucket.combatlog_bucket.id
    
    lambda_function {
        lambda_function_arn = aws_lambda_function.combat_log_reports_lambda.arn
        events = ["s3:ObjectCreated:*"]
        filter_prefix = "form%3DFlush/"
    }

    depends_on = [
        aws_lambda_permission.lambda_combatlog_bucket_permissions
    ]
}

data "aws_s3_bucket" "combatlog_bucket" {
    bucket = var.combatlog_bucket_name
}