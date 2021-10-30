#!/usr/bin/env bash

set -e


# do not redefine builtin `test`
test_() {
    local -r tag="${1}"

    printf "\e[1;32mTesting on %s...\e[0m\n\n" "${tag}"

    docker build \
        --tag="asciinema/asciinema:${tag}" \
        --file="tests/distros/Dockerfile.${tag}" \
        .

    docker run --rm -i "asciinema/asciinema:${tag}" tests/integration.sh
}


test_ ubuntu
test_ debian
test_ fedora
test_ centos

printf "\n\e[1;32mAll tests passed.\e[0m\n"
