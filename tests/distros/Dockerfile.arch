# syntax=docker/dockerfile:1.3

FROM docker.io/library/archlinux:latest

RUN pacman-key --init \
    && pacman --sync --refresh --sysupgrade --noconfirm python3 \
    && printf "LANG=en_US.UTF-8\n" > /etc/locale.conf \
    && locale-gen \
    && pacman --sync --clean --clean --noconfirm

WORKDIR /usr/src/app

COPY asciinema/ asciinema/
COPY tests/ tests/

ENV LANG="en_US.utf8"

USER nobody

ENTRYPOINT ["/bin/bash"]

# vim:ft=dockerfile
