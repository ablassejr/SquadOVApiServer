#!/bin/bash

./vod_processing_worker --config config/config.toml --db $DB_CONNS --threads $PROCESSING_THREADS --pg "postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@127.0.0.1:5432/squadov"