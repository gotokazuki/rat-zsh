.DEFAULT_GOAL := lint
.PHONY: lint lint-md lint-rust lint-zsh fmt test

lint: lint-rust lint-zsh lint-md

fmt:
	cargo fmt --all

test:
	cargo test --all --all-features -q

lint-rust:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings

lint-zsh:
	zsh -n install.zsh

lint-md:
	npx -y markdownlint-cli2 "**/*.md"