.PHONY: cov graph

cov:
	rm -rf target
	cargo kcov --all --open -- \
		--verify \
		--exclude-pattern=${HOME}/.cargo,/usr/lib \
		--exclude-region='#[cfg(test)]:#[cfg(testkcovstopmarker)]'

graph:
	cargo deps | dot -Tpng > target/graph.png
	eog target/graph.png