COMMIT = $(shell git rev-parse --short HEAD)

.PHONY: build test deps fmt fmtdiff travis gox

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
