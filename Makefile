.PHONY: build check run clean bump-patch bump-minor bump-major update-formula

## Build the project (debug)
build:
	cargo build

## Release build (macOS arm64)
release:
	cargo build --release
	zip -j target/release/gsearch_macos_arm64.zip target/release/gsearch
	@echo ""
	@echo "All platform zips ready:"
	@echo "  target/release/gsearch_macos_arm64.zip"

## Type-check without producing a binary
check:
	cargo check

## Run the CLI
run:
	cargo run -- "Weather in San Francisco"

## Remove build artifacts
clean:
	cargo clean

## Bump the patch version (0.1.0 → 0.1.1) and update all version references
bump-patch:
	@old=$$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	major=$$(echo $$old | cut -d. -f1); \
	minor=$$(echo $$old | cut -d. -f2); \
	patch=$$(echo $$old | cut -d. -f3); \
	new="$$major.$$minor.$$((patch+1))"; \
	sed -i '' "s/^version = \"$$old\"/version = \"$$new\"/" Cargo.toml; \
	sed -i '' "s/download\/v$$old/download\/v$$new/" Formula/gsearch.rb; \
	echo "$$old → $$new"

## Bump the minor version (0.1.1 → 0.2.0) and update all version references
bump-minor:
	@old=$$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	major=$$(echo $$old | cut -d. -f1); \
	minor=$$(echo $$old | cut -d. -f2); \
	new="$$major.$$((minor+1)).0"; \
	sed -i '' "s/^version = \"$$old\"/version = \"$$new\"/" Cargo.toml; \
	sed -i '' "s/download\/v$$old/download\/v$$new/" Formula/gsearch.rb; \
	echo "$$old → $$new"

## Bump the major version (0.1.1 → 1.0.0) and update all version references
bump-major:
	@old=$$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	major=$$(echo $$old | cut -d. -f1); \
	new="$$((major+1)).0.0"; \
	sed -i '' "s/^version = \"$$old\"/version = \"$$new\"/" Cargo.toml; \
	sed -i '' "s/download\/v$$old/download\/v$$new/" Formula/gsearch.rb; \
	echo "$$old → $$new"

## Update Formula/gsearch.rb SHA256 from local release zips
## (run after release, before upload)
##   make update-formula
update-formula:
	@mac_zip="target/release/gsearch_macos_arm64.zip"; \
	echo "Computing macOS ARM SHA256 …"; \
	mac_sha=$$(shasum -a 256 "$$mac_zip" | cut -d' ' -f1); \
	echo "macOS ARM SHA256: $$mac_sha"; \
	sed -i '' "s/sha256 \".*\"/sha256 \"$$mac_sha\"/" Formula/gsearch.rb; \
	echo "Formula/gsearch.rb updated"
