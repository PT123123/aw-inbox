# Makefile for ActivityWatch aw-inbox integration test
export CARGO_TARGET_DIR := $(CURDIR)/target

.PHONY: crud-test clean build

kill_port = \
	for port in $(1); do \
		fuser -k $$port/tcp 2>/dev/null || true; \
		pids=$$(lsof -i:$$port -t); if [ -n "$$pids" ]; then kill $$pids; fi; \
	done

crud-test:
	$(call kill_port,5600 5601 5666)
	RUSTFLAGS="-Awarnings" cargo test --test note_crud_test --quiet -- --nocapture 2>&1 | grep -v '^warning:'

clean:
	cargo clean

build:
	cargo build
