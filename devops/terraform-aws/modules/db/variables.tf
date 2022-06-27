variable "postgres_instance_name" {
    type = string
}

variable "postgres_instance_type" {
    type = string
}

variable "postgres_db_subnets" {
    type = list(string)
}

variable "postgres_db_security_groups" {
    type = list(string)
}

variable "postgres_user" {
    type = string
}

variable "postgres_password" {
    type = string
}

variable "postgres_db_size" {
    type = number
}

variable "postgres_max_db_size" {
    type = number
}

variable "redis_instance_type" {
    type = string
}

variable "glue_subnet" {
    type = string
}

variable "secondary_instance_name" {
    type = string
}

variable "secondary_instance_type" {
    type = string
}

variable "secondary_db_size" {
    type = number
}

variable "secondary_max_db_size" {
    type = number
}