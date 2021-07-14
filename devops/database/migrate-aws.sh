#!/bin/bash
OP=$1
DB=$2

flyway \
    -user="$POSTGRES_USER" \
    -password="$POSTGRES_PASSWORD" \
    -url="jdbc:postgresql://$DB/squadov"  \
    -locations="filesystem:$PWD/sql,filesystem:$PWD/prod" \
    -schemas="squadov" \
    $OP