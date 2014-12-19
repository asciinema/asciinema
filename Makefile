NAME=asciinema
VERSION=$(shell grep Version version.go | awk -F '"' '{print $$2}')
COMMIT=$(shell git rev-parse --short HEAD)

DIRS=bin
INSTALL_DIRS=`find $(DIRS) -type d 2>/dev/null`
INSTALL_FILES=`find $(DIRS) -type f 2>/dev/null`
DOC_FILES=*.md LICENSE

PREFIX?=/usr/local
DOC_DIR=$(PREFIX)/share/doc/$(NAME)

.PHONY: build test deps fmt fmtdiff travis gox tag push release install uninstall binary-tarballs os-arch-tgz

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
	for dir in $(INSTALL_DIRS); do mkdir -p $(DESTDIR)$(PREFIX)/$$dir; done
	for file in $(INSTALL_FILES); do cp $$file $(DESTDIR)$(PREFIX)/$$file; done
	mkdir -p $(DESTDIR)$(DOC_DIR)
	cp -r $(DOC_FILES) $(DESTDIR)$(DOC_DIR)/

uninstall:
	for file in $(INSTALL_FILES); do rm -f $(DESTDIR)$(PREFIX)/$$file; done
	rm -rf $(DESTDIR)$(DOC_DIR)

binary-tarballs:
	GOOS=darwin GOARCH=386 $(MAKE) os-arch-tgz
	GOOS=darwin GOARCH=amd64 $(MAKE) os-arch-tgz
	GOOS=linux GOARCH=386 $(MAKE) os-arch-tgz
	GOOS=linux GOARCH=amd64 $(MAKE) os-arch-tgz
	GOOS=linux GOARCH=arm $(MAKE) os-arch-tgz
	cd dist/$(VERSION) && sha1sum *.tar.gz >sha1sum.txt

RELEASE=asciinema-$(VERSION)-$(GOOS)-$(GOARCH)

os-arch-tgz:
	mkdir -p dist/$(VERSION)/$(RELEASE)
	go build -o dist/$(VERSION)/$(RELEASE)/asciinema -ldflags "-X main.GitCommit $(COMMIT)"
	cp README.md CHANGELOG.md LICENSE dist/$(VERSION)/$(RELEASE)
	cd dist/$(VERSION) && tar czf $(RELEASE).tar.gz $(RELEASE)
