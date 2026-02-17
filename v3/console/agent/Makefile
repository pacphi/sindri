# Makefile for the Sindri Console Agent
#
# Usage:
#   make build          – build for the current platform (static)
#   make build-all      – cross-compile all release targets (static)
#   make test           – run unit tests
#   make test-verbose   – run unit tests with verbose output
#   make lint           – run go vet
#   make clean          – remove build artifacts
#
# Binary naming follows the Console extension convention:
#   sindri-agent-<os>-<arch>
#   e.g. sindri-agent-linux-amd64, sindri-agent-darwin-arm64

BINARY   := sindri-agent
VERSION  ?= 0.1.0

# -s -w   : strip debug symbols and DWARF (smaller binary)
# CGO_ENABLED=0 : pure-Go static binary, no libc dependency
LDFLAGS  := -s -w -X main.version=$(VERSION)
GOBUILD  := CGO_ENABLED=0 go build -ldflags "$(LDFLAGS)"

# Output directory
DIST := dist

.PHONY: build build-all test test-verbose lint clean

build:
	@mkdir -p $(DIST)
	CGO_ENABLED=0 go build -ldflags "$(LDFLAGS)" -o $(DIST)/$(BINARY) ./cmd/agent

build-all:
	@mkdir -p $(DIST)
	GOOS=linux  GOARCH=amd64  $(GOBUILD) -o $(DIST)/$(BINARY)-linux-amd64   ./cmd/agent
	GOOS=linux  GOARCH=arm64  $(GOBUILD) -o $(DIST)/$(BINARY)-linux-arm64   ./cmd/agent
	GOOS=darwin GOARCH=amd64  $(GOBUILD) -o $(DIST)/$(BINARY)-darwin-amd64  ./cmd/agent
	GOOS=darwin GOARCH=arm64  $(GOBUILD) -o $(DIST)/$(BINARY)-darwin-arm64  ./cmd/agent
	@echo "Binaries written to $(DIST)/:"
	@ls -lh $(DIST)/

test:
	go test ./... -count=1 -timeout 60s

test-verbose:
	go test ./... -v -count=1 -timeout 60s

lint:
	go vet ./...

clean:
	rm -rf $(DIST)
