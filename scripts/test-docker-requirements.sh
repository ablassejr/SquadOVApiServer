#!/bin/bash
# Test script to verify Docker build requirements

set -e

echo "🧪 Testing Docker build requirements..."

# Test config generation
echo "  ✓ Testing config generation..."
if [ ! -f "config/squadov_local.toml" ]; then
    ./scripts/generate-local-config.sh
fi

if [ -f "config/squadov_local.toml" ]; then
    echo "  ✅ Config file exists: config/squadov_local.toml"
else
    echo "  ❌ Config file missing: config/squadov_local.toml"
    exit 1
fi

# Test AWS profile
if [ -f "devops/aws/local.profile" ]; then
    echo "  ✅ AWS profile exists: devops/aws/local.profile"
else
    echo "  ❌ AWS profile missing: devops/aws/local.profile"
    exit 1
fi

# Test private key
if [ -f "devops/aws/keys/private_s3_vod_cloudfront.pem" ]; then
    echo "  ✅ Private key exists: devops/aws/keys/private_s3_vod_cloudfront.pem"
else
    echo "  ❌ Private key missing: devops/aws/keys/private_s3_vod_cloudfront.pem"
    exit 1
fi

# Test that all required source directories exist
required_dirs=("lib" "server" "tools" "deps" "msa" "lambda" "devops/gcp")
for dir in "${required_dirs[@]}"; do
    if [ -d "$dir" ]; then
        echo "  ✅ Source directory exists: $dir"
    else
        echo "  ❌ Source directory missing: $dir"
        exit 1
    fi
done

# Test that Cargo files exist
if [ -f "Cargo.toml" ] && [ -f "Cargo.lock" ]; then
    echo "  ✅ Cargo files exist"
else
    echo "  ❌ Cargo files missing"
    exit 1
fi

# Test that run script exists
if [ -f "run_api_server.sh" ]; then
    echo "  ✅ Run script exists: run_api_server.sh"
else
    echo "  ❌ Run script missing: run_api_server.sh"
    exit 1
fi

echo ""
echo "🎉 All Docker build requirements satisfied!"
echo ""
echo "You can now build the Docker image using:"
echo "  ./scripts/build-docker-local.sh"
echo ""
echo "Or manually:"
echo "  docker build . --tag squadov_api_server:local --build-arg DEPLOYMENT_ENVIRONMENT=local"