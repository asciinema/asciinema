FROM fedora:26

RUN dnf install -y python3 procps
WORKDIR /usr/src/app
COPY asciinema asciinema
COPY tests tests
ENV LANG en_US.utf8
ENV SHELL /bin/bash
ENV USER docker
