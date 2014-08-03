all: build

build: test
	go build

test:
	go test ./...

fmt:
	find . -type f -name "*.go" | xargs gofmt -d

travis: build fmt
