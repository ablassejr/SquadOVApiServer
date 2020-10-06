#!/bin/bash
DIR=`dirname ${BASH_SOURCE[0]}`
FILE="${DIR}/dev_vars.json"
KEYS=$(jq -r 'keys[]' $FILE)
for K in $KEYS
do
    VAL=$(jq -r ".$K" $FILE)
    export $K="$VAL"
done