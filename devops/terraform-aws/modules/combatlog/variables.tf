variable "combatlog_bucket_name" {
    type = string
}

variable "lambda_subnets" {
    type = list(string)
}

variable "lambda_security_groups" {
    type = list(string)
}

variable "db_host" {
    type = string
}

variable "db_secret" {
    type = string
}