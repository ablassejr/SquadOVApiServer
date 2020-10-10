#!/bin/bash
echo ${GITLAB_REGISTRY_TOKEN} | docker login --username ${GITLAB_USERNAME} --password-stdin registry.gitlab.com
envsubst '${DEPLOYMENT_DOMAIN}' < nginx.conf.tmpl > nginx.conf
envsubst '${DEPLOYMENT_DOMAIN}' < common-tls-options.conf.tmpl > common-tls-options.conf

CONTAINER=registry.gitlab.com/squadov/squadovapiserver/${GCP_PROJECT}/nginx:latest
docker build . --tag ${CONTAINER}
docker push ${CONTAINER}