.PHONY: cov graph

cov:
	rm -rf target
	RUST_TEST_THREADS=1 cargo kcov --all --no-fail-fast --open -- \
		--verify \
		--exclude-pattern=${HOME}/.cargo,/usr/lib \
		--exclude-region='#[cfg(test)]:#[cfg(testkcovstopmarker)]'

graph:
	cargo deps | dot -Tpng > target/graph.png
	eog target/graph.png
