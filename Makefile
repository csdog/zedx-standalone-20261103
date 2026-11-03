.DEFAULT_GOAL := help

ARGS ?= "$(CURDIR)"

.PHONY: help dev build check app

help:
	@printf '%s\n' \
	  'make dev                  Build and run ZedX in the current project' \
	  'make dev ARGS="/path"     Open another project (paths with spaces: ARGS='"'"'"/path with spaces"'"'"')' \
	  'make build                Build the development binary' \
	  'make check                Check compilation without launching' \
	  'make app                  Build and open the local macOS development app'

dev:
	./script/zedx run $(ARGS)

build:
	./script/zedx build

check:
	./script/zedx check

app:
	./script/zedx app $(ARGS)
