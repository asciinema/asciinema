# syntax=docker/dockerfile:1.3

FROM docker.io/library/alpine:3.14

RUN apk --no-cache add bash ca-certificates python3

WORKDIR /usr/src/app

COPY asciinema/ asciinema/
COPY tests/ tests/

ENV LANG="en_US.utf8"

USER nobody

ENTRYPOINT ["/bin/bash"]

# vim:ft=dockerfile
