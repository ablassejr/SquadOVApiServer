#!/bin/bash
CONTAINER=registry.gitlab.com/squadov/squadovapiserver/squadov/nginx_dev:latest
docker build . --tag ${CONTAINER}