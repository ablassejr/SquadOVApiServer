#!/bin/bash
set -xe
DIR=`dirname ${BASH_SOURCE[0]}`
ROOTDIR=`realpath ${DIR}/../../`
COMMIT_HASH=`git rev-parse HEAD`

cd "${ROOTDIR}/devops/docker/nucleus"
envsubst < config.js.tmpl > config.js
TAG=registry.gitlab.com/squadov/squadovapiserver/${GCP_PROJECT}/nucleus:${COMMIT_HASH}
docker build . --tag ${TAG} --build-arg GCP_PROJECT=${GCP_PROJECT}
docker push ${TAG}