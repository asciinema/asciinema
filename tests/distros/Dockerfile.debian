FROM debian:jessie

RUN apt-get update && apt-get install -y \
    ca-certificates \
    locales \
    python3
RUN localedef -i en_US -c -f UTF-8 -A /usr/share/locale/locale.alias en_US.UTF-8
WORKDIR /usr/src/app
COPY asciinema asciinema
COPY tests tests
ENV LANG en_US.utf8
ENV SHELL /bin/bash
ENV USER docker
