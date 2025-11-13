.DEFAULT_GOAL := lint
.PHONY: lint lint-md lint-rust lint-zsh fmt test bump-version-patch bump-version-minor bump-version-major

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

bump-version-patch:
	cargo set-version --bump patch
	@echo "✅ Bumped patch version → $$(grep '^version' Cargo.toml | head -1 | cut -d'\"' -f2)"

bump-version-minor:
	cargo set-version --bump minor
	@echo "✅ Bumped minor version → $$(grep '^version' Cargo.toml | head -1 | cut -d'\"' -f2)"

bump-version-major:
	cargo set-version --bump major
	@echo "✅ Bumped major version → $$(grep '^version' Cargo.toml | head -1 | cut -d'\"' -f2)"

gen-icons:
	aseprite -b assets/icons/rat-zsh.aseprite --scale 1 --save-as assets/icons/rat-zsh.png
	aseprite -b assets/icons/rat-zsh.aseprite --scale 2 --save-as assets/icons/rat-zsh@2x.png
	aseprite -b assets/icons/rat-zsh.aseprite --scale 4 --save-as assets/icons/rat-zsh@4x.png
