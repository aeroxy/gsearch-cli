mod auth;
mod api;
mod grounding;

use std::io::{stdout, IsTerminal};

use anyhow::{Context, Result};
use clap::Parser;
use auth::TokenManager;
use api::ApiClient;
use grounding::format_response;

#[derive(Parser)]
#[command(name = "gsearch", author, version, about, long_about = None)]
struct Cli {
    /// The search query
    query: Vec<String>,

    /// Run the OAuth login flow
    #[arg(short, long)]
    login: bool,

    /// Manual login without opening a browser automatically
    #[arg(long)]
    no_browser: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let token_manager = TokenManager::new()?;
    
    if cli.login {
        token_manager.run_login_flow(cli.no_browser).await?;
        return Ok(());
    }

    if cli.query.is_empty() {
        println!("Usage: gsearch <QUERY> or gsearch --login [--no-browser]");
        return Ok(());
    }
    let query = cli.query.join(" ");

    let (token, project_id) = match token_manager.get_token().await? {
        Some((t, p)) => (t, p),
        None => {
            eprintln!("No valid OAuth token found. Running login flow automatically...");
            token_manager.run_login_flow(cli.no_browser).await?;
            token_manager.get_token().await?.context("Failed to get token after login.")?
        }
    };

    let api_client = ApiClient::new(token, project_id);
    
    println!("Searching for: \"{}\"...", query);
    
    match api_client.search(&query).await {
        Ok(response) => {
            let formatted = format_response(response);
            println!();
            if stdout().is_terminal() {
                termimad::print_text(&formatted);
            } else {
                println!("{}", formatted);
            }
        }
        Err(e) => {
            eprintln!("Error performing search: {:?}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
