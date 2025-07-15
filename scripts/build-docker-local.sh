#!/bin/bash
# Build Docker image for local development

set -e

# Generate local config if it doesn't exist
if [ ! -f "config/squadov_local.toml" ]; then
    echo "Generating local configuration..."
    ./scripts/generate-local-config.sh
fi

echo "Building Docker image for local development..."
docker build . --tag squadov_api_server:local --build-arg DEPLOYMENT_ENVIRONMENT=local

echo "✅ Docker image built successfully as 'squadov_api_server:local'"
echo ""
echo "To run the container:"
echo "  docker run -p 8080:8080 squadov_api_server:local"
echo ""
echo "To run with docker-compose (recommended for development):"
echo "  cd devops/docker"
echo "  docker-compose -f local-dev-compose.yml up"