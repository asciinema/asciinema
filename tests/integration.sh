#!/bin/bash

set -e
set -x

export ASCIINEMA_CONFIG_HOME=`mktemp -d 2>/dev/null || mktemp -d -t asciinema-config-home`
TMP_DATA_DIR=`mktemp -d 2>/dev/null || mktemp -d -t asciinema-data-dir`
trap "echo rm -rf $ASCIINEMA_CONFIG_HOME $TMP_DATA_DIR" EXIT

function asciinema() {
    python3 -m asciinema "$@"
}

asciinema -h

asciinema --version

asciinema auth

asciinema play -s 5 tests/vim.json

asciinema play -s 5 -i 0.2 tests/vim.json

asciinema rec -c who "$TMP_DATA_DIR/1.cast"

bash -c "sleep 1; pkill -28 -n -f 'm asciinema'" &
asciinema rec -c 'bash -c "echo t3st; sleep 2; echo ok"' "$TMP_DATA_DIR/2.cast"

bash -c "sleep 1; pkill -n -f 'bash -c echo t3st'" &
asciinema rec -c 'bash -c "echo t3st; sleep 2; echo ok"' "$TMP_DATA_DIR/3.cast"

bash -c "sleep 1; pkill -9 -n -f 'bash -c echo t3st'" &
asciinema rec -c 'bash -c "echo t3st; sleep 2; echo ok"' "$TMP_DATA_DIR/4.cast"

asciinema rec -i -c 'bash -c "echo t3st; sleep 1; echo ok"' "$TMP_DATA_DIR/5.cast"
