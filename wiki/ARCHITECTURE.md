# Directory Structure

```text
src/
├── main.rs         # CLI entry point and coordination
├── api.rs          # Gemini API interactions and payload definitions
├── auth.rs         # OAuth 2.0 loopback flow and token management
└── grounding.rs    # Parsing and formatting of citation metadata
```

# Module Breakdown

## 1. `src/main.rs`
- **Purpose**: Parses command-line arguments using `clap`.
- **Flow**:
  1. Checks if the `--login` flag is passed. If so, triggers `TokenManager::run_login_flow()`.
  2. Parses the query. If empty, prints usage.
  3. Retrieves the OAuth token. If expired/missing, it automatically intercepts and runs the login flow.
  4. Calls `ApiClient::search` and hands the result to `grounding::format_response`.

## 2. `src/api.rs`
- **Purpose**: Manages communication with `cloudcode-pa.googleapis.com`.
- **Key Mechanics**:
  - **`resolve_project_id`**: Because the Free Tier of Gemini Code Assist uses a dynamically provisioned Google Cloud Project (e.g., `companion-free-12345`), the CLI cannot just use the user's generic `GOOGLE_CLOUD_PROJECT` env var. This module first calls the `v1internal:loadCodeAssist` endpoint to fetch the correct project ID.
  - **`search`**: Executes the `v1internal:generateContent` request, ensuring `googleSearch` tools are injected into the payload.
  - **Proxy Support**: The `reqwest::Client` is built to conditionally accept invalid certs if `HTTPS_PROXY` is present, allowing it to bypass Zscaler/corporate MITM issues.

## 3. `src/auth.rs`
- **Purpose**: Handles authentication parity with `gemini-cli`.
- **Key Mechanics**:
  - Reads/writes to `~/.gemini/oauth_creds.json`.
  - **`run_login_flow`**: Spawns a local `tokio::net::TcpListener` on an ephemeral port. It opens the user's browser to the Google OAuth consent screen, intercepts the `/oauth2callback`, extracts the code, and swaps it for an access/refresh token pair.
  - **Auto-refresh**: Tokens expire in 1 hour. `get_token` automatically detects expiration and calls the `oauth2.googleapis.com/token` endpoint to refresh the token transparently.

## 4. `src/grounding.rs`
- **Purpose**: Text manipulation and citation injection.
- **Key Mechanics**:
  - The Gemini API returns raw text and an array of `GroundingSupport` objects, which contain UTF-8 byte indices (`start_index` / `end_index`).
  - This module sorts the insertions in descending order (to prevent offset shifts) and injects citation markers like `[1]` into the raw text byte array.
  - Finally, it appends a `Sources:` block mapping the indices to `WebSource` URLs and titles.
