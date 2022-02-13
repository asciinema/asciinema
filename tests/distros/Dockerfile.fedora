# syntax=docker/dockerfile:1.3

FROM registry.fedoraproject.org/fedora:35

RUN dnf install -y make python3 procps && dnf clean all

WORKDIR /usr/src/app

COPY asciinema/ asciinema/
COPY tests/ tests/

ENV LANG="en_US.utf8"
ENV SHELL="/bin/bash"

USER nobody

ENTRYPOINT ["/bin/bash"]
# vim:ft=dockerfile
