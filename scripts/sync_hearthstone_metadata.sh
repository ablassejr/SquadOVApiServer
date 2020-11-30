#!/bin/bash
cloud_sql_proxy -instances=${GCP_PROJECT}:us-central1:${POSTGRES_INSTANCE_NAME}=tcp:5555 &
PROXY_PID=$!

sleep 5

python3 sync_hearthstone_metadata.py --folder $1 --jdbc "postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@127.0.0.1:5555/squadov"

kill -9 $PROXY_PID