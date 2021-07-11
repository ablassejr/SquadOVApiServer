#!/bin/bash
cd $1

export TF_VAR_postgres_user=$POSTGRES_USER
export TF_VAR_postgres_password=$POSTGRES_PASSWORD
export TF_VAR_postgres_instance_name=$POSTGRES_INSTANCE_NAME

DEVICE=$2
CODE=$3
TOKEN=$(aws sts get-session-token --serial-number $DEVICE --token-code $CODE --profile default)
export AWS_ACCESS_KEY_ID=$(echo $TOKEN | jq -r '.Credentials.AccessKeyId')
export AWS_SECRET_ACCESS_KEY=$(echo $TOKEN | jq -r '.Credentials.SecretAccessKey')
export AWS_SESSION_TOKEN=$(echo $TOKEN | jq -r '.Credentials.SessionToken')

terraform init
terraform apply