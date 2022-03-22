use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::fs::read_to_string;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Deserialize)]
struct WebsiteConfig {
    // URL to make requests to
    #[serde(rename = "URL")]
    url: String,

    // Name of the website, used in webhooks
    name: String,

    // Time between requests in millisecons
    interval: u64, // Better to use unsigned integer to add an extra layer of input validation

    // String in HTML that will only appear when product is out of stock
    no_stock_indicator: String,
}

impl WebsiteConfig {
    async fn is_in_stock(&self, client: &Client) -> bool {
        let code = self.get_website(client).await;
        code.map_or(false, |fine| !fine.contains(&self.no_stock_indicator)) // You don't have to do this
    }

    async fn send_webhook(&self, url: &str, client: &Client) -> Result<()> {
        println!("[{}]  Sending webhook", Utc::now());
        let msg = json!({
            "username": "🖥  - Monitor",
            "embeds": [{"title": "Monitor triggered", "color": 1841963,
            "description": format!("The product is available on {}", &self.name),
            "url": &self.url, "footer": {"text": "built by peet with ❤️"}}]
        });
        println!("{}", msg);
        let res = client.post(url).json(&msg).send().await?;
        println!("[{}]  Response: {}", Utc::now(), res.text().await?);
        Ok(())
    }

    async fn get_website(&self, client: &Client) -> Result<String> {
        println!("[{}]  Making request to {}", Utc::now(), self.url);
        let res = client.get(&self.url).send().await?;
        Ok(res.text().await?)
    }
}

#[derive(Deserialize)]
struct Config {
    webhook: String,
    websites: Vec<WebsiteConfig>,
}

fn load_yaml_file(file: &Path) -> Result<Config> {
    let file = read_to_string(file).context("Unable to open config.yaml")?;
    Ok(serde_yaml::from_str(&file)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    const FILE: &str = "config.yaml";
    let config = load_yaml_file(Path::new(FILE))?;
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let webhook = Arc::new(config.webhook);
    for site in config.websites {
        println!("✨ Starting monitor for {}", site.name);
        let webhook = Arc::clone(&webhook);
        let client = client.clone();
        tokio::spawn(async move {
            loop {
                if site.is_in_stock(&client).await {
                    println!("🚀 Is in stock on {}!", site.url);
                    if site.send_webhook(&webhook, &client).await.is_err() {
                        println!("Failed to send webhook");
                    }
                }
                tokio::time::sleep(Duration::from_millis(site.interval)).await;
            }
        });
    }
    Ok(())
}
