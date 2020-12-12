#!/bin/bash

python3 test_duplicate_match_uuid.py --mode hearthstone --session $1 --user $3 --ip $5 &
python3 test_duplicate_match_uuid.py --mode hearthstone --session $2 --user $4 --ip $5 &