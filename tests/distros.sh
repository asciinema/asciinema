#!/usr/bin/env bash

set -euo pipefail

readonly DISTROS=(
    'arch'
    'alpine'
    'centos'
    'debian'
    'fedora'
    'ubuntu'
)

readonly DOCKER='docker'

# do not redefine builtin `test`
test_() {
    local -r tag="${1}"

    local -ra docker_opts=(
        "--tag=asciinema/asciinema:${tag}"
        "--file=tests/distros/Dockerfile.${tag}"
    )

    printf "\e[1;32mTesting on %s...\e[0m\n\n" "${tag}"

    # shellcheck disable=SC2068
    "${DOCKER}" build ${docker_opts[@]} .

    "${DOCKER}" run --rm -it "asciinema/asciinema:${tag}" tests/integration.sh
}


for distro in "${DISTROS[@]}"; do
    test_ "${distro}"
done

printf "\n\e[1;32mAll tests passed.\e[0m\n"
