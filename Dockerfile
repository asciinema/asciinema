FROM ubuntu:18.04

RUN apt-get update && apt-get install -y \
    ca-certificates \
    locales \
    python3 \
    python3-setuptools
RUN localedef -i en_US -c -f UTF-8 -A /usr/share/locale/locale.alias en_US.UTF-8
RUN mkdir /usr/src/app
COPY setup.cfg /usr/src/app
COPY setup.py /usr/src/app
COPY *.md /usr/src/app/
COPY doc/*.md /usr/src/app/doc/
COPY man/asciinema.1 /usr/src/app/man/
COPY asciinema /usr/src/app/asciinema
WORKDIR /usr/src/app
RUN python3 setup.py install
ENV LANG en_US.utf8
ENV SHELL /bin/bash
ENV USER docker
WORKDIR /root
ENTRYPOINT ["/usr/local/bin/asciinema"]
