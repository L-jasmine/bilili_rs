use anyhow::Result;
use bilili_rs::api::{APIClient, UserToken};
use std::fs;

/// 从 token 文件加载 APIClient
pub fn load_client(token_file: &str) -> Result<APIClient> {
    let content = fs::read_to_string(token_file)?;
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
