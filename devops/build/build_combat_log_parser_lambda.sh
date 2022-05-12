#!/bin/bash
set -xe
DIR=`dirname ${BASH_SOURCE[0]}`
ROOTDIR=`realpath ${DIR}/../../`
COMMIT_HASH=`git rev-parse HEAD`

cd "${ROOTDIR}"
TAG=combat_log_parser:${COMMIT_HASH}
docker build . --file Dockerfile.parser --tag ${TAG}

DOCKER_ID=$(docker create ${TAG})
mkdir -p ${ROOTDIR}/devops/aws/lambda/build/parser
docker cp ${DOCKER_ID}:/squadov/target/x86_64-unknown-linux-gnu/release/combat_log_parser ${ROOTDIR}/devops/aws/lambda/build/parser/bootstrap
docker rm -v $DOCKER_ID

cd ${ROOTDIR}/devops/aws/lambda/build/parser
zip parser.zip bootstrap
mv parser.zip ../../