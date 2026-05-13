use anyhow::Result;
use bilili_rs::api::{APIClient, UserToken};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenEntry {
    pub token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[allow(dead_code)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
}

pub type TokensMap = HashMap<String, TokenEntry>;

/// 带有 uid 标识的客户端
pub struct ClientWithUid {
    pub uid: String,
    pub client: APIClient,
}

/// 从 token 文件加载所有 APIClient（TOML 格式）
/// 返回 uid -> client 的映射
pub fn load_all_clients(token_file: &str) -> Result<Vec<ClientWithUid>> {
    let content = fs::read_to_string(token_file)?;

    let map: TokensMap =
        toml::from_str(&content).map_err(|e| anyhow::anyhow!("解析 TOML 失败: {}", e))?;

    let mut result = Vec::new();
    for (uid, entry) in map {
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

        match UserToken::create_from_tokens(&tokens) {
            Ok((token, jar)) => match APIClient::new(token, jar, tokens) {
                Ok(client) => {
                    result.push(ClientWithUid { uid, client });
                }
                Err(e) => {
                    log::warn!("跳过 uid={}: 创建客户端失败: {}", uid, e);
                }
            },
            Err(e) => {
                log::warn!("跳过 uid={}: Token 可能已过期: {}", uid, e);
            }
        }
    }

    if result.is_empty() {
        return Err(anyhow::anyhow!(
            "没有找到有效的 token，请检查 token 文件中的 cookies 是否已过期"
        ));
    }

    log::info!("成功加载 {} 个有效 token", result.len());
    Ok(result)
}

/// 从 TOML token 文件加载 APIClient
/// 通过 uid 参数选择用户
pub fn load_client(token_file: &str, uid: Option<&str>) -> Result<APIClient> {
    let content = fs::read_to_string(token_file)?;

    let map: TokensMap =
        toml::from_str(&content).map_err(|e| anyhow::anyhow!("解析 TOML 失败: {}", e))?;

    let selected_uid = if let Some(uid) = uid {
        uid.to_string()
    } else {
        // 未指定 uid，使用第一个
        map.keys()
            .next()
            .ok_or_else(|| anyhow::anyhow!("TOML 文件中没有 token"))?
            .clone()
    };

    let entry = map
        .get(&selected_uid)
        .ok_or_else(|| anyhow::anyhow!("未找到 uid={} 的 token", selected_uid))?;

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

    log::debug!("从 TOML 加载 uid={} 的 token", selected_uid);
    let (token, jar) = UserToken::create_from_tokens(&tokens)?;
    APIClient::new(token, jar, tokens).map_err(|e| anyhow::anyhow!("创建 API 客户端失败: {}", e))
}
