terraform {
    required_providers {
        aws = {
            source  = "hashicorp/aws"
            version = "~> 3.48"
        }
    }

    backend "s3" {
        bucket = "squadov-aws-tf-dev-state"
        key = "tfstate"
        region = "us-east-2"
        profile = "terraformdev"
    }

    required_version = ">= 1.0.2"
}

provider "aws" {
    region              = "us-east-2"
    profile             = "terraformdev"
    allowed_account_ids = [ 897997503846 ]
}

module "network" {
    source = "../modules/network"
}

module "iam" {
    source = "../modules/iam"
}

module "db" {
    source = "../modules/db"

    postgres_instance_name = var.postgres_instance_name
    postgres_user = var.postgres_user
    postgres_password = var.postgres_password
    postgres_db_size = 20
    postgres_max_db_size = 40
    postgres_instance_type = "db.m6g.large"
    postgres_db_subnets = module.network.database_subnets
    postgres_db_security_groups = module.network.database_security_groups
}

module "storage" {
    source = "../modules/storage"

    bucket_suffix = "-dev"
}

module "k8s" {
    source = "../modules/k8s"

    k8s_subnets = module.network.k8s_subnets
    default_fargate_subnets = module.network.default_fargate_subnets
}