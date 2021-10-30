# syntax=docker/dockerfile:1.3

FROM docker.io/library/ubuntu:20.04

ENV DEBIAN_FRONTENT="noninteractive"

RUN apt-get update \
    && apt-get install -y \
        ca-certificates \
        locales \
        python3 \
    && localedef \
        -i en_US \
        -c \
        -f UTF-8 \
        -A /usr/share/locale/locale.alias \
        en_US.UTF-8 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY asciinema/ asciinema/
COPY tests/ tests/

ENV LANG="en_US.utf8"

USER nobody

ENTRYPOINT ["/bin/bash"]

# vim:ft=dockerfile
