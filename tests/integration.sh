#!/bin/bash

set -e

test() {
  echo "Test: $1"
  eval "python2 src $2 >/dev/null || (echo 'failed' && exit 1)"
}

test "help" "-h"
test "version" "-v"
test "auth" "auth"
