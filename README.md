# gsearch-cli (Gemini Web Search)

A standalone, blazing-fast Google Search CLI powered by the Gemini API. It performs grounded Google Searches and returns results with inline citations and source links.

This project was inspired by the official `gemini-cli` and shares the exact same OAuth authentication mechanisms.

## ✨ Features

* **Zero Config for Gemini CLI Users**: Seamlessly shares the OAuth token (`~/.gemini/oauth_creds.json`) with the official Gemini CLI.
* **Native OAuth Flow**: Includes a built-in loopback server for Google OAuth—no external dependencies required.
* **Auto-Refreshing Tokens**: Automatically refreshes your access token in the background so you stay logged in.
* **Proxy Friendly**: Automatically detects `HTTPS_PROXY` and handles custom corporate certificates (like Zscaler) seamlessly.

## 🚀 Installation

### Homebrew (macOS, recommended)

```bash
brew tap aeroxy/gsearch-cli
brew install gsearch
```

### Cargo (crates.io)

If you have Rust installed, you can install directly from crates.io:

```bash
cargo install gsearch-cli
```

## 💻 Usage

```bash
# Simply type your query (no quotes needed!)
gsearch singapore weather

# Force the OAuth login flow
gsearch --login
```

## Agent skill

`skill/gsearch/SKILL.md` is a Claude Code skill that teaches the agent how to use this binary. Drop it into any Claude Code plugin's `skills/` directory and set `gsearch` to the binary path. The skill covers the search workflow, automatic proxy handling, and OAuth login requirements — everything needed to reliably give your agents web search capabilities without large context overhead.

## 📜 License
MIT
