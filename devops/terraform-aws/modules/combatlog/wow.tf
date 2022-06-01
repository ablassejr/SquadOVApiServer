variable "wow_shards" {
    type = number
}

resource "aws_kinesis_stream" "wow_stream" {
    name = "wow-stream"
    shard_count = var.wow_shards

    retention_period = 24
    encryption_type = "KMS"
    kms_key_id = "alias/aws/kinesis"

    stream_mode_details {
        stream_mode = "PROVISIONED"
    }
}

resource "aws_apigatewayv2_integration" "wow_stream_integration" {
    api_id = aws_apigatewayv2_api.combat_log_gateway.id
    integration_type = "AWS_PROXY"
    integration_subtype = "Kinesis-PutRecord"
    payload_format_version = "1.0"
    credentials_arn = aws_iam_role.combat_log_role.arn

    request_parameters = {
        "StreamName" = "${aws_kinesis_stream.wow_stream.name}"
        "Data" = "$request.body"
        "PartitionKey" = "$request.header.partition"
        "SequenceNumberForOrdering" = "$request.header.sequence"
        "Region" = "us-east-2"
    }
}

resource "aws_apigatewayv2_route" "wow_stream_route" {
    api_id = aws_apigatewayv2_api.combat_log_gateway.id
    route_key = "PUT /wow"
    authorization_type = "AWS_IAM"
    target = "integrations/${aws_apigatewayv2_integration.wow_stream_integration.id}"
}

resource "aws_lambda_function" "wow_combat_log_lambda" {
    function_name = "wow-combat-log-lambda"
    role = aws_iam_role.lambda_role.arn

    filename = "../../aws/lambda/parser.zip"
    source_code_hash = filebase64sha256("../../aws/lambda/parser.zip")

    handler = "not.used"
    memory_size = 512
    package_type = "Zip"
    reserved_concurrent_executions = var.wow_shards * 10
    runtime = "provided.al2"
    timeout = 240

    tags = {
        "lambda" = "wow"
    }

    environment {
        variables = {
            "SQUADOV_AWS_REGION" = "us-east-2"
            "SQUADOV_LAMBDA_DB_SECRET" = var.db_secret
            "SQUADOV_LAMBDA_DBHOST" = var.db_host
            "SQUADOV_COMBAT_LOG_BUCKET" = data.aws_s3_bucket.combatlog_bucket.id
        }
    }

    vpc_config {
        subnet_ids = var.lambda_subnets
        security_group_ids = var.lambda_security_groups
    }
}

resource "aws_lambda_event_source_mapping" "wow_lambda_kinesis" {
    event_source_arn  = aws_kinesis_stream.wow_stream.arn
    function_name     = aws_lambda_function.wow_combat_log_lambda.arn
    starting_position = "LATEST"

    batch_size = 10
    maximum_batching_window_in_seconds = 3
    maximum_record_age_in_seconds = -1
    maximum_retry_attempts = 0
    parallelization_factor = 10
}