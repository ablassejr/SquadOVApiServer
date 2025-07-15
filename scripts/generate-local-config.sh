#!/bin/bash
# Generate local development config from template

set -e

# Set default values for local development
export FUSIONAUTH_API_KEY="${FUSIONAUTH_API_KEY:-local-key}"
export FUSIONAUTH_TENANT_ID="${FUSIONAUTH_TENANT_ID:-local-tenant}"
export FUSIONAUTH_CLIENT_ID="${FUSIONAUTH_CLIENT_ID:-local-client}"
export DATABASE_HOST="${DATABASE_HOST:-postgres}"
export POSTGRES_USER="${POSTGRES_USER:-postgres}"
export POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-password}"
export DEPLOYMENT_DOMAIN="${DEPLOYMENT_DOMAIN:-localhost}"
export GITLAB_ACCESS_TOKEN="${GITLAB_ACCESS_TOKEN:-local-token}"
export GITLAB_PROJECT_ID="${GITLAB_PROJECT_ID:-0}"
export KAFKA_BROKERS="${KAFKA_BROKERS:-localhost:9092}"
export KAFKA_CLIENT_KEY="${KAFKA_CLIENT_KEY:-local-client-key}"
export KAFKA_CLIENT_SECRET="${KAFKA_CLIENT_SECRET:-local-client-secret}"
export KAFKA_SERVER_KEY="${KAFKA_SERVER_KEY:-local-server-key}"
export KAFKA_SERVER_SECRET="${KAFKA_SERVER_SECRET:-local-server-secret}"
export RSO_CLIENT_SECRET="${RSO_CLIENT_SECRET:-local-rso-secret}"

# Generate config file
envsubst < config/config.toml.tmpl > config/squadov_local.toml

echo "Generated config/squadov_local.toml for local development"