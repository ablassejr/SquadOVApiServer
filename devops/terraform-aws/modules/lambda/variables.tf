variable "lambda_subnets" {
    type = list(string)
}

variable "lambda_security_groups" {
    type = list(string)
}

variable "combatlog_firehose" {
    type = string
}

variable "ff14_stream" {
    type = string
}

variable "rabbitmq_url" {
    type = string
}