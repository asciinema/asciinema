#!/bin/bash

set -e
set -x

export ASCIINEMA_CONFIG_HOME=`mktemp -d 2>/dev/null || mktemp -d -t asciinema-config-home`
trap "echo rm -rf $ASCIINEMA_CONFIG_HOME" EXIT

function asciinema() {
    python3 -m asciinema "$@"
}

asciinema -h

asciinema --version

asciinema auth

asciinema rec -c who __tmp.json
rm -f __tmp.json

bash -c "sleep 1; pkill -28 -n -f 'm asciinema'" &
asciinema rec -c 'bash -c "echo t3st; sleep 2; echo ok"' __tmp.json
rm -f __tmp.json

bash -c "sleep 1; pkill -n -f 'bash -c echo t3st'" &
asciinema rec -c 'bash -c "echo t3st; sleep 2; echo ok"' __tmp.json
rm -f __tmp.json

bash -c "sleep 1; pkill -9 -n -f 'bash -c echo t3st'" &
asciinema rec -c 'bash -c "echo t3st; sleep 2; echo ok"' __tmp.json
rm -f __tmp.json
