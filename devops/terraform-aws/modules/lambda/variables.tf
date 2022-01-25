variable "db_secret" {
    type = string
}

variable "db_host" {
    type = string
}

variable "lambda_subnets" {
    type = list(string)
}

variable "lambda_security_groups" {
    type = list(string)
}