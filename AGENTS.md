# Project Overview
`gsearch-cli` is a Rust-based command-line interface that performs grounded Google Web Searches via the Gemini API. It operates on Google's Antigravity developer tier using the same OAuth credentials and daily-sandbox endpoints.

# Code Breakdown
For a detailed architectural breakdown, please read **`wiki/ARCHITECTURE.md`**.

# General Rules for Contributing
1. **Idiomatic Rust**: Use idiomatic Rust. Prefer `anyhow` for error handling.
2. **Minimal Dependencies**: Do not add large dependencies unless strictly necessary.
3. **Authentication**: Any changes to `auth.rs` must remain compatible with the `~/.config/gsearch/antigravity.json` schema used by Antigravity-based clients (such as CLIProxyAPI/claude-proxy).
4. **Proxy Handling**: We explicitly support corporate proxies (like Zscaler). If you add new `reqwest` clients, ensure `.danger_accept_invalid_certs(true)` is conditionally applied when `HTTPS_PROXY` is present.

## Preparing Release

1. Bump the version in `Cargo.toml`.
2. Build the release binary: `cargo build --release`
3. Zip the binary inside the release folder: `zip -j target/release/gsearch_macos_arm64.zip target/release/gsearch`
4. Calculate the SHA256: `shasum -a 256 target/release/gsearch_macos_arm64.zip`
5. Update `Formula/gsearch.rb` with the new version, URL, and SHA256.
