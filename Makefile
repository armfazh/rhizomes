.ONESHELL:

all:
	@echo "Welcome!"

clone_verifier:
 	git clone https://github.com/armfazh/rhizomes --branch rhizomes/verifier verifier

bench_verifier:
	cd verifier
	cargo test --features experimental
	cargo test --features rhizomes
	cargo bench --features experimental --bench speed_tests -- "prio3.*prepare_init" --quiet --save-baseline baseline
	cargo bench --features rhizomes     --bench speed_tests -- "prio3.*prepare_init" --quiet --save-baseline rhizomes
	critcmp baseline rhizomes

graph_verifier:
	cd verifier
	critcmp --export baseline > baseline.json
	critcmp --export rhizomes > rhizomes.json
	python3 ../graph.py baseline.json rhizomes.json ../comparison_verifier.png
	rm baseline.json rhizomes.json