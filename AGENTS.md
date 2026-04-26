# Project Overview
`gsearch-cli` is a Rust-based command-line interface that performs grounded Google Web Searches via the Gemini API. It shares the exact same OAuth credential flow as the official Node.js `gemini-cli`.

# Code Breakdown
For a detailed architectural breakdown, please read **`wiki/ARCHITECTURE.md`**.

# General Rules for Contributing
1. **Idiomatic Rust**: Use idiomatic Rust. Prefer `anyhow` for error handling.
2. **Minimal Dependencies**: Do not add large dependencies unless strictly necessary.
3. **Authentication**: Any changes to `auth.rs` must remain compatible with the `~/.gemini/oauth_creds.json` schema used by Google's official tools.
4. **Proxy Handling**: We explicitly support corporate proxies (like Zscaler). If you add new `reqwest` clients, ensure `.danger_accept_invalid_certs(true)` is conditionally applied when `HTTPS_PROXY` is present.
