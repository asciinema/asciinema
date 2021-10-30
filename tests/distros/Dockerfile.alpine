# syntax=docker/dockerfile:1.3

FROM docker.io/library/alpine:3.14

# https://github.com/actions/runner/issues/241
RUN apk --no-cache add bash ca-certificates make python3 util-linux

WORKDIR /usr/src/app

COPY asciinema/ asciinema/
COPY tests/ tests/

ENV LANG="en_US.utf8"

USER nobody

ENTRYPOINT ["/bin/bash"]

# vim:ft=dockerfile
