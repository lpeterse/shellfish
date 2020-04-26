.PHONY: run test build kcov deps example clean

run:
	cargo build --release --target "x86_64-unknown-linux-musl" --example rssh-client
	RUST_LOG=debug ./target/x86_64-unknown-linux-musl/release/examples/rssh-client

test:
	cargo test

build:
	cargo build

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
	RUST_LOG=debug cargo run --release --target "x86_64-unknown-linux-musl" --example client
