use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct TokenEntry {
    token: String,
    deadline: String,
}

type TokensMap = BTreeMap<String, TokenEntry>;

/// 获取 fingerprint (buvid3/buvid4)
async fn fetch_fingerprint(client: &reqwest::Client) -> Result<(String, String)> {
    let resp = client
        .get("https://api.bilibili.com/x/frontend/finger/spi")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "application/json, text/plain, */*")
        .header("Referer", "https://www.bilibili.com")
        .header("Origin", "https://www.bilibili.com")
        .send()
        .await?;

    let finger_json: serde_json::Value = resp.json().await?;

    match finger_json.get("data") {
        Some(data) => {
            let b_3 = data.get("b_3").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let b_4 = data.get("b_4").and_then(|v| v.as_str()).unwrap_or("").to_string();
            Ok((b_3, b_4))
        }
        None => {
            Err(anyhow::anyhow!("fingerprint API 返回格式异常"))
        }
    }
}

/// 为单个 token 添加 buvid3/buvid4
fn add_fingerprint_to_token(token: &str, buvid3: &str, buvid4: &str) -> String {
    let mut lines: Vec<String> = token
        .lines()
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect();

    // 移除旧的 buvid3/buvid4
    lines.retain(|line| !line.starts_with("buvid3=") && !line.starts_with("buvid4="));

    // 添加新的 buvid3/buvid4
    if !buvid3.is_empty() {
        lines.insert(
            0,
            format!("buvid3={}; Path=/; Domain=.bilibili.com; Max-Age=2147483647", buvid3),
        );
    }
    if !buvid4.is_empty() {
        lines.insert(
            1,
            format!("buvid4={}; Path=/; Domain=.bilibili.com; Max-Age=2147483647", buvid4),
        );
    }

    lines.join("\n")
}

pub async fn run_refresh_token(token_file: String, uid: Option<&str>) -> Result<()> {
    log::info!("刷新 token 文件: {}", token_file);

    let content = fs::read_to_string(&token_file)?;
    let mut map: TokensMap = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("解析 TOML 失败: {}", e))?;

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    if let Some(target_uid) = uid {
        // 只刷新指定的 token
        log::info!("正在刷新 uid={} 的 token...", target_uid);
        if let Some(entry) = map.get_mut(target_uid) {
            let (buvid3, buvid4) = fetch_fingerprint(&client).await?;
            if buvid3.is_empty() && buvid4.is_empty() {
                log::warn!("未获取到 buvid3/buvid4");
                return Ok(());
            }
            log::info!("获取到 buvid3: {}", buvid3);
            log::info!("获取到 buvid4: {}", buvid4);
            entry.token = add_fingerprint_to_token(&entry.token, &buvid3, &buvid4);
            let new_content = toml::to_string_pretty(&map)?;
            fs::write(&token_file, new_content)?;
            println!("Token 刷新成功! 已为 uid={} 添加 buvid3 和 buvid4", target_uid);
        } else {
            log::warn!("未找到 uid={} 的 token", target_uid);
            return Err(anyhow::anyhow!("未找到 uid={} 的 token", target_uid));
        }
    } else {
        // 刷新所有 token - 每个 token 获取独立的 fingerprint
        log::info!("正在刷新 {} 个 token...", map.len());
        let keys: Vec<String> = map.keys().cloned().collect();
        for (i, key) in keys.iter().enumerate() {
            log::info!("[{}/{}] 正在刷新 uid={}...", i + 1, keys.len(), key);
            match fetch_fingerprint(&client).await {
                Ok((buvid3, buvid4)) => {
                    if buvid3.is_empty() && buvid4.is_empty() {
                        log::warn!("uid={} 未获取到 buvid3/buvid4，跳过", key);
                        continue;
                    }
                    if let Some(entry) = map.get_mut(key) {
                        entry.token = add_fingerprint_to_token(&entry.token, &buvid3, &buvid4);
                    }
                }
                Err(e) => {
                    log::warn!("uid={} 获取 fingerprint 失败: {}，跳过", key, e);
                }
            }
        }
        let new_content = toml::to_string_pretty(&map)?;
        fs::write(&token_file, new_content)?;
        println!("Token 刷新成功! 已为 {} 个 token 添加 buvid3 和 buvid4", map.len());
    }

    Ok(())
}
