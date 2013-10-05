#!/bin/bash

set -e

test() {
  echo "Test: $1"
  eval "PYTHONPATH=. python -m asciinema.__main__ $2 >/dev/null || (echo 'failed' && exit 1)"
}

test "help" "-h"
test "version" "-v"
test "auth" "auth"
