.PHONY: cov

cov:
	rm -rf target
	cargo kcov --all --open -- \
		--verify \
		--exclude-pattern=${HOME}/.cargo,/usr/lib \
		--exclude-region='#[cfg(test)]:#[cfg(testkcovstopmarker)]'
