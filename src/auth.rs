use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const OAUTH_CLIENT_ID: &str = "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
pub const OAUTH_CLIENT_SECRET: &str = "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";
pub const OAUTH_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiAuth {
    pub access_token: String,
    pub scope: String,
    pub token_type: String,
    pub id_token: Option<String>,
    pub expiry_date: Option<u64>,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
    scope: String,
    token_type: String,
    id_token: Option<String>,
}

pub struct TokenManager {
    token_path: PathBuf,
}

impl TokenManager {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let gemini_dir = home.join(".gemini");
        if !gemini_dir.exists() {
            fs::create_dir_all(&gemini_dir)?;
        }
        let token_path = gemini_dir.join("oauth_creds.json");
        Ok(Self { token_path })
    }

    pub fn load_gemini_auth(&self) -> Result<Option<GeminiAuth>> {
        if !self.token_path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(&self.token_path)
            .context("Failed to read oauth_creds.json")?;
        let auth: GeminiAuth = serde_json::from_str(&data)
            .context("Failed to parse oauth_creds.json")?;
        Ok(Some(auth))
    }

    pub async fn get_token(&self) -> Result<Option<String>> {
        if let Some(auth) = self.load_gemini_auth()? {
            if !self.is_expired(&auth) {
                return Ok(Some(auth.access_token));
            }
            if let Some(refresh_token) = &auth.refresh_token {
                let new_auth = self.refresh_access_token(refresh_token).await?;
                return Ok(Some(new_auth.access_token));
            }
        }
        Ok(None)
    }

    fn is_expired(&self, auth: &GeminiAuth) -> bool {
        if let Some(expiry_date) = auth.expiry_date {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let buffer = 5 * 60 * 1000;
            return now + buffer >= expiry_date;
        }
        false
    }

    async fn refresh_access_token(&self, refresh_token: &str) -> Result<GeminiAuth> {
        let mut builder = reqwest::Client::builder();
        if std::env::var("HTTPS_PROXY").is_ok() || std::env::var("https_proxy").is_ok() {
            builder = builder.danger_accept_invalid_certs(true);
        }
        let client = builder.build()?;
        let params = [
            ("client_id", OAUTH_CLIENT_ID),
            ("client_secret", OAUTH_CLIENT_SECRET),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let res = client.post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            anyhow::bail!("Failed to refresh token: {}", error_text);
        }

        let token_res: TokenResponse = res.json().await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let expiry_date = now + (token_res.expires_in * 1000);

        let auth = GeminiAuth {
            access_token: token_res.access_token,
            scope: token_res.scope,
            token_type: token_res.token_type,
            id_token: token_res.id_token,
            expiry_date: Some(expiry_date),
            refresh_token: token_res.refresh_token.or_else(|| Some(refresh_token.to_string())),
        };

        let auth_json = serde_json::to_string_pretty(&auth)?;
        fs::write(&self.token_path, auth_json)?;
        Ok(auth)
    }

    pub async fn run_login_flow(&self) -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let redirect_uri = format!("http://127.0.0.1:{}/oauth2callback", port);
        
        let state = format!("{:x}", rand::random::<u128>());

        let auth_url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&state={}",
            OAUTH_CLIENT_ID,
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(OAUTH_SCOPE),
            state
        );

        println!("Google login required for Gemini Web Search.");
        println!("Attempting to open authentication page in your browser...");
        println!("Otherwise navigate to:\n\n{}\n", auth_url);

        let _ = open::that(&auth_url);

        println!("Waiting for authentication...");

        let (mut stream, _) = listener.accept().await?;
        let mut buffer = [0; 4096];
        let bytes_read = stream.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);

        let mut code = String::new();
        let mut received_state = String::new();

        if let Some(first_line) = request.lines().next() {
            if let Some(path) = first_line.split_whitespace().nth(1) {
                if let Some(query) = path.split('?').nth(1) {
                    for pair in query.split('&') {
                        let mut parts = pair.split('=');
                        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                            if key == "code" {
                                code = value.to_string();
                            } else if key == "state" {
                                received_state = value.to_string();
                            }
                        }
                    }
                }
            }
        }

        let response = if code.is_empty() || received_state != state {
            "HTTP/1.1 301 Moved Permanently\r\nLocation: https://developers.google.com/gemini-code-assist/auth_failure_gemini\r\n\r\n"
        } else {
            "HTTP/1.1 301 Moved Permanently\r\nLocation: https://developers.google.com/gemini-code-assist/auth_success_gemini\r\n\r\n"
        };
        
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;

        if code.is_empty() {
            anyhow::bail!("Authorization failed or was cancelled.");
        }
        if received_state != state {
            anyhow::bail!("State mismatch. Possible CSRF attack.");
        }

        self.exchange_code_for_token(code, redirect_uri).await?;
        println!("Authentication successful!");

        Ok(())
    }

    async fn exchange_code_for_token(&self, code: String, redirect_uri: String) -> Result<()> {
        let mut builder = reqwest::Client::builder();
        if std::env::var("HTTPS_PROXY").is_ok() || std::env::var("https_proxy").is_ok() {
            builder = builder.danger_accept_invalid_certs(true);
        }
        let client = builder.build()?;
        let params = [
            ("client_id", OAUTH_CLIENT_ID),
            ("client_secret", OAUTH_CLIENT_SECRET),
            ("code", &code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &redirect_uri),
        ];

        let res = client.post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            anyhow::bail!("Failed to exchange token: {}", error_text);
        }

        let token_res: TokenResponse = res.json().await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let expiry_date = now + (token_res.expires_in * 1000);

        let auth = GeminiAuth {
            access_token: token_res.access_token,
            scope: token_res.scope,
            token_type: token_res.token_type,
            id_token: token_res.id_token,
            expiry_date: Some(expiry_date),
            refresh_token: token_res.refresh_token,
        };

        let auth_json = serde_json::to_string_pretty(&auth)?;
        fs::write(&self.token_path, auth_json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.token_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.token_path, perms)?;
        }

        Ok(())
    }
}
