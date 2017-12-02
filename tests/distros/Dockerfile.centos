FROM centos:7

RUN yum install -y epel-release
RUN yum install -y python34
WORKDIR /usr/src/app
COPY asciinema asciinema
COPY tests tests
ENV LANG en_US.utf8
ENV SHELL /bin/bash
ENV USER docker
