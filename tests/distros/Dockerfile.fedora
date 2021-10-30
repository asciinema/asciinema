# syntax=docker/dockerfile:1.3

# https://medium.com/nttlabs/ubuntu-21-10-and-fedora-35-do-not-work-on-docker-20-10-9-1cd439d9921
# https://www.mail-archive.com/ubuntu-bugs@lists.ubuntu.com/msg5971024.html
FROM registry.fedoraproject.org/fedora:34

RUN dnf install -y python3 procps && dnf clean all

WORKDIR /usr/src/app

COPY asciinema/ asciinema/
COPY tests/ tests/

ENV LANG="en_US.utf8"
ENV SHELL="/bin/bash"

USER nobody

ENTRYPOINT ["/bin/bash"]
# vim:ft=dockerfile
