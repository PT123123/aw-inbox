# Makefile for ActivityWatch aw-inbox integration test

.PHONY: test-crud

test-crud:
	RUSTFLAGS="-Awarnings" cargo test --test note_crud_test --quiet -- --nocapture 2>&1 | grep -v '^warning:'
