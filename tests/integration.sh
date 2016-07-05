#!/bin/bash

set -e
set -x

python3 -m asciinema -h

python3 -m asciinema --version

python3 -m asciinema auth

python3 -m asciinema rec -c who __tmp.json && rm __tmp.json

bash -c "sleep 1; pkill -28 -f 'thon -m asciinema'" &
python3 -m asciinema rec -c 'bash -c "echo t3st; sleep 2; echo ok"' __tmp.json && rm __tmp.json

bash -c "sleep 1; pkill -f 'bash -c echo t3st'" &
python3 -m asciinema rec -c 'bash -c "echo t3st; sleep 2; echo ok"' __tmp.json && rm __tmp.json

bash -c "sleep 1; pkill -9 -f 'bash -c echo t3st'" &
python3 -m asciinema rec -c 'bash -c "echo t3st; sleep 2; echo ok"' __tmp.json && rm __tmp.json
