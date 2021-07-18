#!/bin/bash
set -xe
DIR=`dirname ${BASH_SOURCE[0]}`
ROOTDIR=`realpath ${DIR}/../../`
COMMIT_HASH=`git rev-parse HEAD`

cd "${ROOTDIR}/config"
envsubst < config.toml.tmpl > squadov_${DEPLOYMENT_ENVIRONMENT}.toml

cd "${ROOTDIR}"
TAG=registry.gitlab.com/squadov/squadovapiserver/${DEPLOYMENT_ENVIRONMENT}/squadov_api_server:${COMMIT_HASH}
docker build . --tag ${TAG} --build-arg DEPLOYMENT_ENVIRONMENT=${DEPLOYMENT_ENVIRONMENT}
docker push ${TAG}