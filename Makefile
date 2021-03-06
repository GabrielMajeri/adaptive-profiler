.SILENT:

.PHONY: all lint build run

all: | build run

lint:
	cargo fmt
	cargo clippy

check:
	cargo check

build:
	maturin develop

run:
	python3 ./benchmark.py
