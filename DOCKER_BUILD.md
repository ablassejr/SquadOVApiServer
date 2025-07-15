# Docker Build Guide

This document explains how to build Docker images for the SquadOV API Server.

## Quick Start (Local Development)

For local development, use the provided script:

```bash
./scripts/build-docker-local.sh
```

This script will:
1. Generate the required configuration file from the template
2. Build the Docker image with local development settings
3. Tag the image as `squadov_api_server:local`

## Manual Build

### Prerequisites

1. Ensure you have the required configuration and key files:
   - `config/squadov_local.toml` (generated from template)
   - `devops/aws/local.profile` (AWS credentials for local dev)
   - `devops/aws/keys/private_s3_vod_cloudfront.pem` (CloudFront private key)

### Generate Configuration

```bash
# Generate local development config from template
./scripts/generate-local-config.sh
```

### Build the Image

```bash
# Build for local development
docker build . --tag squadov_api_server:local --build-arg DEPLOYMENT_ENVIRONMENT=local

# Build for production (requires production environment variables)
export DEPLOYMENT_ENVIRONMENT=production
cd config && envsubst < config.toml.tmpl > squadov_${DEPLOYMENT_ENVIRONMENT}.toml && cd ..
docker build . --tag squadov_api_server:production --build-arg DEPLOYMENT_ENVIRONMENT=production
```

## Production Build

For production builds, use the existing build script:

```bash
export DEPLOYMENT_ENVIRONMENT=production
# Set other required environment variables...
./devops/build/build.sh
```

## Environment-Specific Files

The Docker build requires different files based on the deployment environment:

- **Config**: `config/squadov_${DEPLOYMENT_ENVIRONMENT}.toml`
- **AWS Profile**: `devops/aws/${DEPLOYMENT_ENVIRONMENT}.profile`
- **Private Key**: `devops/aws/keys/private_s3_vod_cloudfront.pem`

For local development, placeholder files are provided. For production, ensure you have the actual credentials and keys.

## Running the Container

### Standalone
```bash
docker run -p 8080:8080 squadov_api_server:local
```

### With Dependencies (Recommended)
```bash
cd devops/docker
docker-compose -f local-dev-compose.yml up
```

The docker-compose setup includes PostgreSQL, Elasticsearch, and FusionAuth dependencies.

## Troubleshooting

### Missing Files
If you get "file not found" errors during build:
1. Run `./scripts/generate-local-config.sh` to create the config file
2. Ensure `devops/aws/local.profile` exists (created automatically)
3. Ensure `devops/aws/keys/private_s3_vod_cloudfront.pem` exists (placeholder provided for local dev)

### Network Issues
If package installation fails:
1. Check internet connectivity
2. Try using a different DNS server
3. Use Docker build with `--network=host` if behind a proxy

### Build Cache
To force a fresh build:
```bash
docker build --no-cache . --tag squadov_api_server:local --build-arg DEPLOYMENT_ENVIRONMENT=local
```