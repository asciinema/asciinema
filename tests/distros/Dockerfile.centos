# syntax=docker/dockerfile:1.3

FROM docker.io/library/centos:8

RUN yum install -y epel-release && yum install -y make python39 && yum clean all

WORKDIR /usr/src/app

COPY asciinema/ asciinema/
COPY tests/ tests/

ENV LANG="en_US.utf8"

USER nobody

ENTRYPOINT ["/bin/bash"]

# vim:ft=dockerfile
