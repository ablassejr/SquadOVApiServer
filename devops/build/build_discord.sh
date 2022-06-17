#!/bin/bash
set -xe
DIR=`dirname ${BASH_SOURCE[0]}`
ROOTDIR=`realpath ${DIR}/../../`
COMMIT_HASH=`git rev-parse HEAD`

cd "${ROOTDIR}/config"
envsubst < discord.toml.tmpl > discord.toml

cd "${ROOTDIR}"
TAG=registry.gitlab.com/squadov/squadovapiserver/${DEPLOYMENT_ENVIRONMENT}/discord:${COMMIT_HASH}
docker build . --file Dockerfile.discord --tag ${TAG} --build-arg DEPLOYMENT_ENVIRONMENT=${DEPLOYMENT_ENVIRONMENT}
docker push ${TAG}