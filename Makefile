all: build

deps:
	go get -d -v ./...

build: test
	go build -o bin/asciinema

test:
	go test ./...

fmt:
	find . -type f -name "*.go" | xargs gofmt -d

travis: build fmt

gox:
	gox -os="darwin linux" -arch="386 amd64" -output="bin/asciinema_{{.OS}}_{{.Arch}}"
