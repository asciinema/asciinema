NAME=asciinema
VERSION=$(shell grep Version version.go | awk -F '"' '{print $$2}')
COMMIT=$(shell git rev-parse --short HEAD)

DIRS=bin
INSTALL_DIRS=`find $(DIRS) -type d 2>/dev/null`
INSTALL_FILES=`find $(DIRS) -type f 2>/dev/null`
DOC_FILES=*.md LICENSE

PREFIX?=/usr/local
DOC_DIR=$(PREFIX)/share/doc/$(NAME)

.PHONY: build test deps fmt fmtdiff travis gox tag push release

all: build

build: test
	go build -o bin/asciinema -ldflags "-X main.GitCommit $(COMMIT)"

test:
	go test ./...

deps:
	go get -d -v ./...

fmt:
	go fmt ./...

fmtdiff:
	find . -type f -name "*.go" | xargs gofmt -d

travis: build fmtdiff

gox:
	gox -os="darwin linux" -arch="386 amd64" -output="bin/asciinema_{{.OS}}_{{.Arch}}" -ldflags "-X main.GitCommit $(COMMIT)"

tag:
	git tag | grep "v$(VERSION)" && echo "Tag v$(VERSION) exists" && exit 1 || true
	git tag -s -m "Releasing $(VERSION)" v$(VERSION)
	git push --tags

push:
	echo "TODO: uploading binaries to github release"

release: test tag push

install:
	for dir in $(INSTALL_DIRS); do mkdir -p $(PREFIX)/$$dir; done
	for file in $(INSTALL_FILES); do cp $$file $(PREFIX)/$$file; done
	mkdir -p $(DOC_DIR)
	cp -r $(DOC_FILES) $(DOC_DIR)/

uninstall:
	for file in $(INSTALL_FILES); do rm -f $(PREFIX)/$$file; done
	rm -rf $(DOC_DIR)
