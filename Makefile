.PHONY: build test clean deps example example-server

build:
	cargo build

test:
	cargo test

clean:
	cargo clean
	rm -rf target

deps:
	cargo deps | dot -Tpng > target/graph.png
	eog target/graph.png

example:
	RUST_LOG=debug cargo run --release --example shellfish-proxy -- -v -v socks5 localhost

example-server:
	RUST_LOG=debug cargo run --release --example shellfish-server
