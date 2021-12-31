.PHONY: run test build kcov deps example example-dev clean

build:
	cargo build

test:
	cargo test

clean:
	cargo clean
	rm -rf target

kcov: clean
	RUST_TEST_THREADS=1 cargo kcov --all --no-fail-fast --open -- \
		--verify \
		--exclude-pattern=${HOME}/.cargo,${HOME}/.rustup,/usr/lib \
		--exclude-region='#[cfg(test)]:#[cfg(testkcovstopmarker)]'

deps:
	cargo deps | dot -Tpng > target/graph.png
	eog target/graph.png

example:
	cargo build --release --target "x86_64-unknown-linux-musl" --example shellfish-proxy
	./target/x86_64-unknown-linux-musl/release/examples/shellfish-proxy -v -v socks5 localhost

example-dev:
	cargo run --release --example shellfish-proxy -- -v -v socks5 localhost

example-server:
	RUST_LOG=debug cargo run --release --example shellfish-server
