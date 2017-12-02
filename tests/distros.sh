#!/usr/bin/env bash

set -e

path_to_self="${BASH_SOURCE[0]}"
tests_dir="$(cd "$(dirname "$path_to_self")" && pwd)"

test() {
    printf "\e[1;32mTesting on $1...\e[0m\n"
    echo

    docker build -t asciinema/asciinema:$1 -f tests/distros/Dockerfile.$1 .
    docker run --rm -ti asciinema/asciinema:$1 tests/integration.sh
}

test ubuntu
test debian
test fedora
test centos

echo
printf "\e[1;32mAll tests passed.\e[0m\n"
