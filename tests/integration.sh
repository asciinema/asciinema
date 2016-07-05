#!/bin/bash

set -e
set -x

python3 -m asciinema -h
python3 -m asciinema --version
python3 -m asciinema auth
python3 -m asciinema rec -c who __who.json && rm __who.json
python3 -m asciinema rec -c 'bash -c "echo 1; sleep 1; echo 2"' __sleep.json && rm __sleep.json
