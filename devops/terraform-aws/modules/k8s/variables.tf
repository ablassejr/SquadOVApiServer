variable "public_k8s_subnets" {
    type = list(string)
}

variable "private_k8s_subnets" {
    type = list(string)
}

variable "default_fargate_subnets" {
    type = list(string)
}