use anyhow::Result;

use crate::client::TokensMap;

pub async fn run_refresh_username(token_file: String, uid: Option<&str>) -> Result<()> {
    let content = std::fs::read_to_string(&token_file)?;
    let mut map: TokensMap =
        toml::from_str(&content).map_err(|e| anyhow::anyhow!("解析 TOML 失败: {}", e))?;

    let uids_to_refresh: Vec<String> = if let Some(uid) = uid {
        if !map.contains_key(uid) {
            return Err(anyhow::anyhow!("未找到 uid={} 的 token", uid));
        }
        vec![uid.to_string()]
    } else {
        map.keys().cloned().collect()
    };

    let mut updates: Vec<(String, String)> = Vec::new();
    let is_batch = uids_to_refresh.len() > 1;

    for (i, uid) in uids_to_refresh.iter().enumerate() {
        if is_batch && i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        let entry = match map.get(uid) {
            Some(e) => e,
            None => continue,
        };

        let tokens: Vec<String> = entry
            .token
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

        let (token, jar) = match bilili_rs::api::UserToken::create_from_tokens(&tokens) {
            Ok(t) => t,
            Err(e) => {
                log::warn!("跳过 uid={}: {}", uid, e);
                continue;
            }
        };

        let client = match bilili_rs::api::APIClient::new(token, jar, tokens) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("跳过 uid={}: {}", uid, e);
                continue;
            }
        };

        let uid_num: u64 = match uid.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };

        match client.get_user_info(uid_num).await {
            Ok(result) if result.code == 0 => {
                if let Some(info) = result.data {
                    println!("uid={} -> {}", uid, info.name);
                    updates.push((uid.clone(), info.name));
                }
            }
            Ok(result) => {
                log::warn!("uid={} 查询失败: {:?}", uid, result.message);
            }
            Err(e) => {
                log::warn!("uid={} 请求失败: {}", uid, e);
            }
        }
    }

    if updates.is_empty() {
        println!("没有更新");
        return Ok(());
    }

    for (uid, name) in updates {
        if let Some(entry) = map.get_mut(&uid) {
            entry.username = Some(name);
        }
    }

    let new_content =
        toml::to_string_pretty(&map).map_err(|e| anyhow::anyhow!("序列化 TOML 失败: {}", e))?;
    std::fs::write(&token_file, new_content)?;

    println!("已保存到 {}", token_file);
    Ok(())
}
