#!/bin/bash
cloud_sql_proxy -instances=${GCP_PROJECT}:us-central1:${POSTGRES_INSTANCE_NAME}=tcp:5555 &
PROXY_PID=$!

sleep 5

flyway \
    -user="$POSTGRES_USER" \
    -password="$POSTGRES_PASSWORD" \
    -url="jdbc:postgresql://127.0.0.1:5555/squadov"  \
    -locations="filesystem:$PWD/sql" \
    -schemas="squadov" \
    migrate

kill -9 $PROXY_PID