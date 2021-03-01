#!/bin/bash
echo ${GITLAB_REGISTRY_TOKEN} | docker login --username ${GITLAB_USERNAME} --password-stdin registry.gitlab.com
envsubst '${DEPLOYMENT_DOMAIN}' < nginx.conf.tmpl > nginx.conf
COMMIT_HASH=`git rev-parse HEAD`

CONTAINER=registry.gitlab.com/squadov/squadovapiserver/${GCP_PROJECT}/landing_nginx:${COMMIT_HASH}
docker build . --tag ${CONTAINER}
docker push ${CONTAINER}