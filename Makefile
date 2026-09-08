SHELL=/bin/bash

.PHONY: help
help:
	@awk -F':.*##' '/^[-_a-zA-Z0-9]+:.*##/{printf"%-12s\t%s\n",$$1,$$2}' $(MAKEFILE_LIST) | sort

.PHONY: format
format: format-clj format-gha format-rust ## format
	npx prettier -w .
.PHONY: format-clj
format-clj:
	cljstyle fix
.PHONY: format-gha
format-gha:
	pinact run
.PHONY: format-rust
format-rust:
	cargo fmt

.PHONY: install
install: ## segzifyをinstallする
	cargo install --path . --locked

.PHONY: lint
lint: lint-clj lint-gha ## lint
.PHONY: lint-clj
lint-clj:
	cljstyle check
	cljstyle find | xargs -t clj-kondo --parallel --lint
.PHONY: lint-gha
lint-gha:
	yamllint .github/workflows/
	actionlint
	zizmor .
	ghalint run

.PHONY: test
test: ## test
	cargo test
