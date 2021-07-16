#!/bin/bash
set -xe
DIR=`dirname ${BASH_SOURCE[0]}`
ROOTDIR=`realpath ${DIR}/../../`
COMMIT_HASH=`git rev-parse HEAD`

cd "${ROOTDIR}/config"
envsubst < config.toml.tmpl > squadov_${DEPLOYMENT_ENVIRONMENT}.toml

cd "${ROOTDIR}"
TAG=registry.gitlab.com/squadov/squadovapiserver/${DEPLOYMENT_ENVIRONMENT}/vod_processor:${COMMIT_HASH}
docker build . --file Dockerfile.vod --tag ${TAG} --build-arg DEPLOYMENT_ENVIRONMENT=${DEPLOYMENT_ENVIRONMENT}
docker push ${TAG}