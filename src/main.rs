mod logging;
mod config;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time;

const CHECK_INTERVAL: u64 = 300;        // 5 mins

#[derive(Serialize, Deserialize, Debug)]
struct CloudflareRecord {
    id: String,
    content: String,
    #[serde(rename = "type")]
    record_type: String,
}

#[derive(Deserialize)]
struct CFResponse {
    result: Vec<CloudflareRecord>,
}

async fn get_public_ip(v6: bool) -> Result<String> {
    let url = if v6 { "https://api64.ipify.org" } else { "https://api.ipify.org" };
    let resp = reqwest::get(url).await?.text().await?;
    Ok(resp)
}

async fn update_cf(v6: bool, current_ip: &str, cf_api_token: &str, zone_id: &str, domain: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let rtype = if v6 { "AAAA" } else { "A" };
    
    // get existing record
    let list_url = format!("https://api.cloudflare.com/client/v4/zones/{}/dns_records?name={}&type={}", zone_id, domain, rtype);
    let res: CFResponse = client.get(&list_url)
        .bearer_auth(cf_api_token)
        .send().await?
        .json().await?;

    if let Some(record) = res.result.first() {
        if record.content == current_ip {
            tracing::info!("[{}] IP 未变动，跳过更新", rtype);
            return Ok(());
        }

        // update record
        let update_url = format!("https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}", zone_id, record.id);
        let payload = serde_json::json!({
            "type": rtype,
            "name": domain,
            "content": current_ip,
            "ttl": 60
        });

        client.put(&update_url)
            .bearer_auth(cf_api_token)
            .json(&payload)
            .send().await?;
        tracing::info!("[{}] 已成功更新至: {}", rtype, current_ip);
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let _logging_guard = logging::init_logging("logs", "ddns", "info");
    tracing::info!("DDNS 服务启动");

    let config = match config::ApiConfig::load_from_file("config.toml") {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!("加载配置文件失败: {}", e);
            return;
        }
    };

    let cf_api_token = &config.api_token;
    let zone_id = &config.zone_id;
    let domain = &config.domain;

    let mut interval = time::interval(Duration::from_secs(CHECK_INTERVAL));

    loop {
        interval.tick().await;
        
        if let Ok(ip4) = get_public_ip(false).await {
            let _ = update_cf(false, &ip4, cf_api_token, zone_id, domain).await;
        }
        if let Ok(ip6) = get_public_ip(true).await {
            let _ = update_cf(true, &ip6, cf_api_token, zone_id, domain).await;
        }
    }
}