#!/bin/bash
set -xe
DIR=`dirname ${BASH_SOURCE[0]}`
ROOTDIR=`realpath ${DIR}/../../`
COMMIT_HASH=`git rev-parse HEAD`

cd "${ROOTDIR}/config"
envsubst < config.toml.tmpl > ${GCP_PROJECT}.toml

cd "${ROOTDIR}"
TAG=registry.gitlab.com/squadov/squadovapiserver/${GCP_PROJECT}/vod_processor:${COMMIT_HASH}
docker build . --file Dockerfile.vod --tag ${TAG} --build-arg GCP_PROJECT=${GCP_PROJECT}
docker push ${TAG}