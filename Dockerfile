# syntax=docker/dockerfile:1.3

FROM docker.io/library/ubuntu:22.04

ENV DEBIAN_FRONTEND="noninteractive"

RUN apt-get update \
    && apt-get install -y \
        ca-certificates \
        locales \
        python3 \
        python3-pip \
    && localedef \
        -i en_US \
        -c \
        -f UTF-8 \
        -A /usr/share/locale/locale.alias \
        en_US.UTF-8

COPY pyproject.toml setup.cfg *.md /usr/src/app/
COPY doc/*.md /usr/src/app/doc/
COPY man/asciinema.1 /usr/src/app/man/
COPY asciinema/ /usr/src/app/asciinema/
COPY README.md LICENSE /usr/src/app/

WORKDIR /usr/src/app

RUN pip3 install .

WORKDIR /root

ENV LANG="en_US.utf8"
ENV SHELL="/bin/bash"

ENTRYPOINT ["/usr/local/bin/asciinema"]
CMD ["--help"]

# vim:ft=dockerfile
