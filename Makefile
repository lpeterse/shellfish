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
#	eog target/graph.png

example:
	RUST_LOG=debug cargo run --release --example shellfish-proxy -- -v -v socks5 localhost

example-server:
	RUST_LOG=debug cargo run --release --example shellfish-server

raspi:
	cargo build --release --example shellfish-server --target armv7-unknown-linux-musleabihf
	ssh 192.168.0.150 pkill shellfish || true
	scp target/armv7-unknown-linux-musleabihf/release/examples/shellfish-server 192.168.0.150:
	ssh 192.168.0.150 SSH_AUTH_SOCK= RUST_LOG=debug ./shellfish-server
