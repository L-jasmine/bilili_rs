use anyhow::Result;
use std::fs;

pub async fn run_refresh_token(token_file: String) -> Result<()> {
    log::info!("刷新 token 文件: {}", token_file);

    // 读取现有 token 文件
    let content = fs::read_to_string(&token_file)?;
    let mut lines: Vec<String> = content
        .split('\n')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect();

    // 调用 fingerprint API 获取 buvid3 和 buvid4
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let resp = client
        .get("https://api.bilibili.com/x/frontend/finger/spi")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await?;

    let finger_json: serde_json::Value = resp.json().await?;

    if let Some(data) = finger_json.get("data") {
        let b_3 = data.get("b_3").and_then(|v| v.as_str()).unwrap_or("");
        let b_4 = data.get("b_4").and_then(|v| v.as_str()).unwrap_or("");

        if !b_3.is_empty() || !b_4.is_empty() {
            log::info!("获取到 buvid3: {}", b_3);
            log::info!("获取到 buvid4: {}", b_4);

            // 移除旧的 buvid3/buvid4
            lines.retain(|line| {
                !line.starts_with("buvid3=") && !line.starts_with("buvid4=")
            });

            // 添加新的 buvid3/buvid4（带 Domain 属性）
            if !b_3.is_empty() {
                lines.insert(0, format!("buvid3={}; Path=/; Domain=.bilibili.com; Max-Age=2147483647", b_3));
            }
            if !b_4.is_empty() {
                lines.insert(1, format!("buvid4={}; Path=/; Domain=.bilibili.com; Max-Age=2147483647", b_4));
            }

            // 写回文件
            let new_content = lines.join("\n");
            fs::write(&token_file, new_content)?;

            println!("Token 刷新成功! 已添加 buvid3 和 buvid4");
        } else {
            log::warn!("未获取到 buvid3/buvid4");
        }
    } else {
        log::warn!("fingerprint API 返回格式异常");
    }

    Ok(())
}
