use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::fs;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub const OAUTH_CLIENT_ID: &str = "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
pub const OAUTH_CLIENT_SECRET: &str = "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";
pub const OAUTH_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform \
                               https://www.googleapis.com/auth/userinfo.email \
                               https://www.googleapis.com/auth/userinfo.profile \
                               https://www.googleapis.com/auth/cclog \
                               https://www.googleapis.com/auth/experimentsandconfigs";

const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v2/userinfo?alt=json";
const CODE_ASSIST_DAILY: &str = "https://daily-cloudcode-pa.googleapis.com";
const CODE_ASSIST_VERSION: &str = "v1internal";

const CALLBACK_PORT: u16 = 51121;
const CALLBACK_PATH: &str = "/oauth-callback";
const ANTIGRAVITY_USER_AGENT: &str = "antigravity/cli/1.0.9 darwin/arm64";

const LOGIN_TIMEOUT_SECS: u64 = 300;
const ONBOARD_TIMEOUT_SECS: u64 = 30;
const EXPIRY_BUFFER_MS: u64 = 60_000;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AntigravityAuth {
    pub r#type: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub timestamp: u64,
    pub expired: String,
    pub email: String,
    pub project_id: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    // Google's refresh-token grant does not return a refresh_token; only the
    // initial authorization_code exchange does. Keep this optional.
    refresh_token: Option<String>,
    expires_in: u64,
}

pub struct TokenManager {
    token_path: PathBuf,
}

impl TokenManager {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let config_dir = home.join(".config").join("gsearch");
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        let token_path = config_dir.join("antigravity.json");
        Ok(Self { token_path })
    }

    pub fn load_auth(&self) -> Result<Option<AntigravityAuth>> {
        if !self.token_path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(&self.token_path)
            .context("Failed to read antigravity.json")?;
        let auth: AntigravityAuth = serde_json::from_str(&data)
            .context("Failed to parse antigravity.json")?;
        Ok(Some(auth))
    }

    pub async fn get_token(&self) -> Result<Option<(String, String)>> {
        if let Some(auth) = self.load_auth()? {
            if !self.is_expired(&auth) {
                return Ok(Some((auth.access_token, auth.project_id)));
            }
            let new_auth = self.refresh_access_token(&auth).await?;
            return Ok(Some((new_auth.access_token, new_auth.project_id)));
        }
        Ok(None)
    }

    fn is_expired(&self, auth: &AntigravityAuth) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let expires_at_ms = auth.timestamp + auth.expires_in * 1000;
        now + EXPIRY_BUFFER_MS >= expires_at_ms
    }

    async fn refresh_access_token(&self, auth: &AntigravityAuth) -> Result<AntigravityAuth> {
        let mut builder = reqwest::Client::builder();
        if std::env::var("HTTPS_PROXY").is_ok() || std::env::var("https_proxy").is_ok() {
            builder = builder.danger_accept_invalid_certs(true);
        }
        let client = builder.build()?;
        
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &auth.refresh_token),
            ("client_id", OAUTH_CLIENT_ID),
            ("client_secret", OAUTH_CLIENT_SECRET),
        ];

        let res = client.post(TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            anyhow::bail!("Failed to refresh token: {}", error_text);
        }

        let token_res: TokenResponse = res.json().await.context("parse refresh token response")?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let expired = rfc3339_from_now(token_res.expires_in);

        let new_auth = AntigravityAuth {
            r#type: "antigravity".to_string(),
            access_token: token_res.access_token,
            refresh_token: token_res
                .refresh_token
                .unwrap_or_else(|| auth.refresh_token.clone()),
            expires_in: token_res.expires_in,
            timestamp: now,
            expired,
            email: auth.email.clone(),
            project_id: auth.project_id.clone(),
        };

        self.save_auth(&new_auth)?;
        Ok(new_auth)
    }

    fn save_auth(&self, auth: &AntigravityAuth) -> Result<()> {
        let auth_json = serde_json::to_string_pretty(auth)?;
        let tmp_path = self.token_path.with_extension("tmp");
        fs::write(&tmp_path, auth_json)?;
        fs::rename(&tmp_path, &self.token_path)?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.token_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.token_path, perms)?;
        }
        Ok(())
    }

    pub async fn run_login_flow(&self, no_browser: bool) -> Result<()> {
        let mut builder = reqwest::Client::builder();
        if std::env::var("HTTPS_PROXY").is_ok() || std::env::var("https_proxy").is_ok() {
            builder = builder.danger_accept_invalid_certs(true);
        }
        let client = builder.build()?;

        let (code, redirect_uri) = if no_browser {
            self.manual_oauth().await?
        } else {
            self.loopback_oauth().await?
        };

        println!("Exchanging authorization code for tokens...");
        let token = self.exchange_code(&client, &code, &redirect_uri).await?;
        
        println!("Fetching account details...");
        let email = self.fetch_email(&client, &token.access_token).await?;
        println!("Authenticated as {}", email);

        println!("Resolving Cloud project ID...");
        let project_id = self.fetch_project_id(&client, &token.access_token).await?;
        println!("Using project ID: {}", project_id);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let expired = rfc3339_from_now(token.expires_in);

        let auth = AntigravityAuth {
            r#type: "antigravity".to_string(),
            access_token: token.access_token,
            refresh_token: token
                .refresh_token
                .context("Login response missing refresh_token")?,
            expires_in: token.expires_in,
            timestamp: now,
            expired,
            email,
            project_id,
        };

        self.save_auth(&auth)?;
        println!("Saved credentials to {}", self.token_path.display());

        Ok(())
    }

    async fn loopback_oauth(&self) -> Result<(String, String)> {
        let (listener, bound_port) = match TcpListener::bind(("127.0.0.1", CALLBACK_PORT)).await {
            Ok(l) => (l, CALLBACK_PORT),
            Err(e) => {
                eprintln!("Port {} is unavailable ({}); falling back to a random port", CALLBACK_PORT, e);
                let l = TcpListener::bind(("127.0.0.1", 0)).await?;
                let port = l.local_addr()?.port();
                (l, port)
            }
        };

        let host = if bound_port == CALLBACK_PORT { "localhost" } else { "127.0.0.1" };
        let redirect_uri = format!("http://{host}:{bound_port}{CALLBACK_PATH}");
        let state = format!("{:x}", rand::random::<u64>());

        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&state={}",
            AUTH_ENDPOINT,
            OAUTH_CLIENT_ID,
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(OAUTH_SCOPE),
            state
        );

        println!("Opening browser for sign-in. If it doesn't open, visit:\n\n  {}\n", auth_url);
        let _ = open::that(&auth_url);

        println!("Waiting for authentication...");
        
        let code = tokio::time::timeout(
            Duration::from_secs(LOGIN_TIMEOUT_SECS),
            self.accept_oauth_callback(&listener, &state)
        )
        .await
        .context("OAuth flow timed out")??;

        Ok((code, redirect_uri))
    }

    async fn accept_oauth_callback(&self, listener: &TcpListener, expected_state: &str) -> Result<String> {
        loop {
            let (mut stream, _) = listener.accept().await?;
            let path = {
                let mut reader = BufReader::new(&mut stream);
                match tokio::time::timeout(Duration::from_secs(5), async {
                    let mut first_line = String::new();
                    reader.read_line(&mut first_line).await?;
                    let mut line = String::new();
                    while let Ok(n) = reader.read_line(&mut line).await {
                        if n == 0 || line == "\r\n" || line == "\n" {
                            break;
                        }
                        line.clear();
                    }
                    Ok::<_, std::io::Error>(first_line)
                })
                .await
                {
                    Ok(Ok(first_line)) => first_line.split_whitespace().nth(1).unwrap_or("").to_string(),
                    _ => String::new(),
                }
            };

            if let Some(error) = extract_query_param(&path, "error") {
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                     <html><body><h2>Authentication Failed</h2>\
                     <p>Error: {}</p>\
                     <p>You can close this window.</p></body></html>",
                    error
                );
                let _ = stream.write_all(response.as_bytes()).await;
                anyhow::bail!("OAuth error: {}", error);
            }

            let received_state = extract_query_param(&path, "state").unwrap_or_default();
            if !received_state.is_empty() && received_state != expected_state {
                let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\n\r\n\
                                <html><body><h2>Authentication Failed</h2>\
                                <p>Error: CSRF state mismatch.</p></body></html>";
                let _ = stream.write_all(response.as_bytes()).await;
                anyhow::bail!("CSRF state mismatch.");
            }

            if let Some(code) = extract_query_param(&path, "code") {
                let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                     <html><body><h2>Authentication Successful!</h2>\
                     <p>You can close this window and return to your terminal.</p></body></html>";
                let _ = stream.write_all(response.as_bytes()).await;
                return Ok(code);
            }

            let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n").await;
        }
    }

    async fn manual_oauth(&self) -> Result<(String, String)> {
        let redirect_uri = format!("http://localhost:{}{}", CALLBACK_PORT, CALLBACK_PATH);
        let state = format!("{:x}", rand::random::<u64>());

        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&state={}",
            AUTH_ENDPOINT,
            OAUTH_CLIENT_ID,
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(OAUTH_SCOPE),
            state
        );

        println!("--- Antigravity Sign-In ---");
        println!("Please visit the following URL in any browser to authorize:\n\n  {}\n", auth_url);
        println!("Note: Since this is a manual flow, after authorizing in your browser,");
        println!("your browser will show a 'Connection Error' page (e.g. unable to connect to localhost:{}).", CALLBACK_PORT);
        println!("This is expected! Please copy the full URL from your browser's address bar");
        println!("and paste it below.\n");

        let code = tokio::time::timeout(
            Duration::from_secs(LOGIN_TIMEOUT_SECS),
            async {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let stdin = tokio::io::stdin();
                let mut reader = BufReader::new(stdin);
                
                loop {
                    print!("Paste the authorization code or redirect URL: ");
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    
                    let mut line = String::new();
                    reader.read_line(&mut line).await.context("read from stdin")?;
                    let code_or_url = line.trim();
                    if code_or_url.is_empty() {
                        continue;
                    }
                    
                    if code_or_url.contains("error=") {
                        anyhow::bail!("OAuth authentication failed (error param detected)");
                    }

                    if let Some(code) = extract_query_param(code_or_url, "code") {
                        return Ok(code);
                    }

                    if code_or_url.contains(' ') || code_or_url.contains('\n') {
                        println!("Malformed input — code cannot contain whitespace. Please try again.\n");
                        continue;
                    }

                    return Ok(code_or_url.to_string());
                }
            }
        )
        .await
        .context("OAuth flow timed out waiting for input")??;

        Ok((code, redirect_uri))
    }

    async fn exchange_code(&self, client: &reqwest::Client, code: &str, redirect_uri: &str) -> Result<TokenResponse> {
        let params = [
            ("code", code),
            ("client_id", OAUTH_CLIENT_ID),
            ("client_secret", OAUTH_CLIENT_SECRET),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let res = client.post(TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await
            .context("token exchange request")?;

        if !res.status().is_success() {
            let status = res.status();
            let error_text = res.text().await.unwrap_or_default();
            anyhow::bail!("Token exchange failed ({status}): {error_text}");
        }

        let token_res: TokenResponse = res.json().await.context("parse token response")?;
        Ok(token_res)
    }

    async fn fetch_email(&self, client: &reqwest::Client, access_token: &str) -> Result<String> {
        let res: Value = client.get(USERINFO_ENDPOINT)
            .bearer_auth(access_token)
            .send()
            .await
            .context("userinfo request")?
            .json()
            .await
            .context("parse userinfo response")?;

        let email = res["email"]
            .as_str()
            .map(|s| s.to_string())
            .context("UserInfo response had no email field")?;
        Ok(email)
    }

    async fn fetch_project_id(&self, client: &reqwest::Client, access_token: &str) -> Result<String> {
        let metadata = json!({ "ideType": "ANTIGRAVITY" });
        let load_body = json!({ "metadata": metadata });

        let url = format!("{}/{}:loadCodeAssist", CODE_ASSIST_DAILY, CODE_ASSIST_VERSION);
        let res = client.post(&url)
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .header("User-Agent", ANTIGRAVITY_USER_AGENT)
            .json(&load_body)
            .send()
            .await
            .context("loadCodeAssist request")?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("loadCodeAssist failed ({status}): {text}");
        }

        let load_resp: Value = res.json().await.context("parse loadCodeAssist response")?;

        let tier_id = load_resp
            .get("allowedTiers")
            .and_then(|t| t.as_array())
            .and_then(|tiers| {
                tiers.iter().find(|t| t.get("isDefault").and_then(|d| d.as_bool()).unwrap_or(false))
            })
            .and_then(|t| t.get("id").and_then(|i| i.as_str()))
            .unwrap_or("legacy-tier")
            .to_string();

        let mut project_id = extract_project_from_val(&load_resp).unwrap_or_default();

        if project_id.is_empty() {
            println!("No active managed project found. Auto-provisioning via onboardUser...");
            let onboard_body = json!({ "tierId": tier_id, "metadata": metadata });
            project_id = self.onboard_poll(client, &onboard_body, access_token).await?.unwrap_or_default();
        }

        if project_id.is_empty() {
            anyhow::bail!("Could not determine or auto-provision a Google Cloud Project for Antigravity.");
        }

        let finalize_body = json!({
            "tierId": tier_id,
            "metadata": metadata,
            "cloudaicompanionProject": project_id,
        });

        if let Ok(Some(p)) = self.onboard_poll(client, &finalize_body, access_token).await {
            if !p.is_empty() {
                project_id = p;
            }
        }

        Ok(project_id)
    }

    async fn onboard_poll(&self, client: &reqwest::Client, body: &Value, access_token: &str) -> Result<Option<String>> {
        let url = format!("{}/{}:onboardUser", CODE_ASSIST_DAILY, CODE_ASSIST_VERSION);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(ONBOARD_TIMEOUT_SECS);

        loop {
            let res = client.post(&url)
                .bearer_auth(access_token)
                .header("Content-Type", "application/json")
                .header("User-Agent", ANTIGRAVITY_USER_AGENT)
                .json(body)
                .send()
                .await
                .context("onboardUser request")?;

            if !res.status().is_success() {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                anyhow::bail!("onboardUser failed ({status}): {text}");
            }

            let resp: Value = res.json().await.context("parse onboardUser response")?;

            if resp.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                let project = resp.get("response").and_then(extract_project_from_val);
                return Ok(project);
            }

            if tokio::time::Instant::now() >= deadline {
                return Ok(None);
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

fn extract_project_from_val(v: &Value) -> Option<String> {
    v.get("cloudaicompanionProject").and_then(|p| {
        if let Some(s) = p.as_str() {
            let s = s.trim();
            return (!s.is_empty()).then(|| s.to_string());
        }
        p.get("id")
            .and_then(|i| i.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

fn rfc3339_from_now(expires_in: u64) -> String {
    let expires_duration = chrono::Duration::try_seconds(expires_in as i64).unwrap_or_else(chrono::Duration::zero);
    (chrono::Utc::now() + expires_duration)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn extract_query_param(path: &str, key: &str) -> Option<String> {
    let query = path.split('?').nth(1)?;
    query.split('&').find_map(|param| {
        let (k, v) = param.split_once('=')?;
        if k == key {
            Some(percent_decode(v))
        } else {
            None
        }
    })
}

fn percent_decode(input: &str) -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(input.len());
    let mut iter = input.bytes();
    while let Some(b) = iter.next() {
        match b {
            b'%' => {
                let h1 = iter.next();
                let h2 = iter.next();
                let decoded = match (h1, h2) {
                    (Some(d1), Some(d2)) => {
                        (d1 as char).to_digit(16).zip((d2 as char).to_digit(16))
                    }
                    _ => None,
                };
                match decoded {
                    Some((v1, v2)) => bytes.push((v1 << 4 | v2) as u8),
                    None => {
                        bytes.push(b'%');
                        if let Some(x) = h1 { bytes.push(x); }
                        if let Some(x) = h2 { bytes.push(x); }
                    }
                }
            }
            b'+' => bytes.push(b' '),
            other => bytes.push(other),
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
