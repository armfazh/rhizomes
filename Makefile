all: test

test:
	cargo test --features experimental
	cargo test --features rhizomes -- --skip vec

bench:
	cargo bench --features experimental --bench speed_tests -- prio3 --quiet --save-baseline baseline
	cargo bench --features rhizomes     --bench speed_tests -- prio3 --quiet --save-baseline rhizomes
	critcmp baseline rhizomes

graph:
	critcmp --export baseline > baseline.json
	critcmp --export rhizomes > rhizomes.json
	python3 graph.py baseline.json rhizomes.json comparison.png
	rm baseline.json rhizomes.json