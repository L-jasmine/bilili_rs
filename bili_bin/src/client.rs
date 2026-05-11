use anyhow::Result;
use bilili_rs::api::{APIClient, UserToken};
use serde::Deserialize;
use std::fs;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct TokenEntry {
    token: String,
    #[allow(dead_code)]
    deadline: String,
}

type TokensMap = HashMap<String, TokenEntry>;

/// 带有 uid 标识的客户端
pub struct ClientWithUid {
    pub uid: String,
    pub client: APIClient,
}

/// 从 token 文件加载所有 APIClient（仅 TOML 格式）
/// 返回 uid -> client 的映射
pub fn load_all_clients(token_file: &str) -> Result<Vec<ClientWithUid>> {
    let content = fs::read_to_string(token_file)?;

    // 检测是否为 TOML 格式
    if !content.trim().starts_with('[') {
        // 纯文本格式，返回单个客户端（uid 为空）
        let tokens: Vec<String> = content
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

        let (token, jar) = UserToken::create_from_tokens(&tokens)?;
        let client = APIClient::new(token, jar, tokens)
            .map_err(|e| anyhow::anyhow!("创建 API 客户端失败: {}", e))?;
        return Ok(vec![ClientWithUid {
            uid: "default".to_string(),
            client,
        }]);
    }

    // TOML 格式，加载所有 token
    let map: TokensMap = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("解析 TOML 失败: {}", e))?;

    let mut result = Vec::new();
    for (uid, entry) in map {
        let tokens: Vec<String> = entry.token
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

        let (token, jar) = UserToken::create_from_tokens(&tokens)?;
        let client = APIClient::new(token, jar, tokens)
            .map_err(|e| anyhow::anyhow!("创建 uid={} 的 API 客户端失败: {}", uid, e))?;
        result.push(ClientWithUid { uid, client });
    }

    Ok(result)
}

/// 从 token 文件加载 APIClient
/// 支持两种格式：
/// 1. 纯文本格式：每行一个 Set-Cookie 值
/// 2. TOML 格式：包含多个用户的 token，通过 uid 参数选择
pub fn load_client(token_file: &str, uid: Option<&str>) -> Result<APIClient> {
    let content = fs::read_to_string(token_file)?;

    // 检测是否为 TOML 格式（包含 '[' 且能被解析为 TOML）
    if content.trim().starts_with('[') {
        // TOML 格式
        let map: TokensMap = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("解析 TOML 失败: {}", e))?;

        let selected_uid = if let Some(uid) = uid {
            uid.to_string()
        } else {
            // 未指定 uid，使用第一个
            map.keys()
                .next()
                .ok_or_else(|| anyhow::anyhow!("TOML 文件中没有 token"))?
                .clone()
        };

        let entry = map.get(&selected_uid)
            .ok_or_else(|| anyhow::anyhow!("未找到 uid={} 的 token", selected_uid))?;

        // 解析多行 token 字符串
        let tokens: Vec<String> = entry.token
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
        APIClient::new(token, jar, tokens)
            .map_err(|e| anyhow::anyhow!("创建 API 客户端失败: {}", e))
    } else {
        // 纯文本格式（兼容原有逻辑）
        if uid.is_some() {
            log::warn!("纯文本 token 文件不支持 uid 参数，忽略");
        }
        let tokens: Vec<String> = content
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

        let (token, jar) = UserToken::create_from_tokens(&tokens)?;
        APIClient::new(token, jar, tokens)
            .map_err(|e| anyhow::anyhow!("创建 API 客户端失败: {}", e))
    }
}
