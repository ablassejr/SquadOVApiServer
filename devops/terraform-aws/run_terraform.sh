#!/bin/bash
cd $1

export TF_VAR_postgres_user=$POSTGRES_USER
export TF_VAR_postgres_password=$POSTGRES_PASSWORD
export TF_VAR_postgres_instance_name=$POSTGRES_INSTANCE_NAME
export TF_VAR_rabbitmq_url=$RABBITMQ_AMQP_URL

terraform init
terraform apply