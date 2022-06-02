resource "aws_lambda_function" "combat_log_reports_lambda" {
    function_name = "combat-log-reports-lambda"
    role = aws_iam_role.lambda_role.arn

    filename = "../../aws/lambda/reports.zip"
    source_code_hash = filebase64sha256("../../aws/lambda/reports.zip")

    handler = "not.used"
    memory_size = 128
    package_type = "Zip"
    reserved_concurrent_executions = 256
    runtime = "provided.al2"
    timeout = 360

    tags = {
        "lambda" = "combatlog-reports"
    }

    vpc_config {
        subnet_ids = var.lambda_subnets
        security_group_ids = var.lambda_security_groups
    }

    ephemeral_storage {
        size = 10240
    }

    environment {
        variables = {
            "SQUADOV_AWS_REGION" = "us-east-2"
            "SQUADOV_EFS_DIRECTORY" = "/tmp"
            "SQUADOV_LAMBDA_DB_SECRET" = var.db_secret
            "SQUADOV_LAMBDA_DBHOST" = var.db_host
            "SQUADOV_AMQP_URL" = var.amqp_url
            "SQUADOV_ES_RABBITMQ_QUEUE" = "squadov_elasticsearch"
        }
    }

    layers = [
        "arn:aws:lambda:us-east-2:580247275435:layer:LambdaInsightsExtension:18"
    ]
}