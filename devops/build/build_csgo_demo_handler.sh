#!/bin/bash
set -xe
DIR=`dirname ${BASH_SOURCE[0]}`
ROOTDIR=`realpath ${DIR}/../../`
COMMIT_HASH=`git rev-parse HEAD`

cd "${ROOTDIR}/config"
envsubst < csgo_demo_handler_config.toml.tmpl > csgo_demo_handler_config.toml

cd "${ROOTDIR}"
TAG=registry.gitlab.com/squadov/squadovapiserver/${DEPLOYMENT_ENVIRONMENT}/csgo_demo_handler:${COMMIT_HASH}
docker build . --file Dockerfile.csgodemo --tag ${TAG}
docker push ${TAG}