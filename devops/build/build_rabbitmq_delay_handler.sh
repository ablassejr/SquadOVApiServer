#!/bin/bash
set -xe
DIR=`dirname ${BASH_SOURCE[0]}`
ROOTDIR=`realpath ${DIR}/../../`
COMMIT_HASH=`git rev-parse HEAD`

cd "${ROOTDIR}/config"
envsubst < rabbitmq_delay_config.toml.tmpl > rabbitmq_delay_config.toml

cd "${ROOTDIR}"
TAG=registry.gitlab.com/squadov/squadovapiserver/${GCP_PROJECT}/rabbitmq_delay_handler:${COMMIT_HASH}
docker build . --file Dockerfile.rmqdelay --tag ${TAG} --build-arg GCP_PROJECT=${GCP_PROJECT}
docker push ${TAG}