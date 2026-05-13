use anyhow::Result;
use bilili_rs::api::LoginUrl;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

const STATE_FILE: &str = ".bili_login_state";

#[derive(Debug, Serialize, Deserialize)]
struct TokenEntry {
    token: String,
    deadline: String,
}

type TokensMap = BTreeMap<String, TokenEntry>;

/// 保存 token 到 TOML 格式
/// 如果文件已存在，则更新对应 uid 的条目；否则创建新文件
fn save_toml_token(output: &str, uid: &str, cookies: &[String]) -> Result<()> {
    use time::format_description::well_known::Iso8601;

    // 从 SESSDATA 中提取过期时间戳
    let sessdata_expire = cookies
        .iter()
        .find(|c| c.starts_with("SESSDATA="))
        .and_then(|c| c.split(',').nth(1).and_then(|s| s.parse::<i64>().ok()));

    let deadline = if let Some(timestamp) = sessdata_expire {
        // 转换为本地时间的 ISO 8601 格式
        let utc = time::OffsetDateTime::from_unix_timestamp(timestamp)?;
        // 转换为系统本地时区
        let local = utc.to_offset(time::UtcOffset::current_local_offset()?);
        local.format(&Iso8601::DEFAULT)?
    } else {
        // 默认 30 天后（本地时间）
        let now_local =
            time::OffsetDateTime::now_utc().to_offset(time::UtcOffset::current_local_offset()?);
        let dt = now_local
            .checked_add(time::Duration::days(30))
            .ok_or(anyhow::anyhow!("计算过期时间失败"))?;
        dt.format(&Iso8601::DEFAULT)?
    };

    let token_content = cookies.join("\n");
    let entry = TokenEntry {
        token: token_content,
        deadline,
    };

    // 检查文件是否存在
    if std::path::Path::new(output).exists() {
        // 读取现有 TOML，更新条目
        let content = fs::read_to_string(output)?;
        let mut map: TokensMap =
            toml::from_str(&content).map_err(|e| anyhow::anyhow!("解析 TOML 失败: {}", e))?;
        map.insert(uid.to_string(), entry);
        let new_content = toml::to_string_pretty(&map)?;
        fs::write(output, new_content)?;
    } else {
        // 创建新文件
        let mut map = TokensMap::new();
        map.insert(uid.to_string(), entry);
        let content = toml::to_string_pretty(&map)?;
        fs::write(output, content)?;
    }

    Ok(())
}

/// 在终端显示二维码
pub fn display_qrcode(url: &str) -> Result<()> {
    let qrcode = qrcode::QrCode::new(url)?;
    let image = qrcode.render::<qrcode::render::unicode::Dense1x2>().build();
    println!("\n请使用哔哩哔哩手机App扫描以下二维码登录:\n");
    println!("{}", image);
    println!("\n二维码链接: {}\n", url);
    Ok(())
}

/// 生成 SVG 二维码文件
pub fn save_qrcode_svg(url: &str, path: &str) -> Result<()> {
    use qrcode::render::svg;

    let qrcode = qrcode::QrCode::new(url)?;
    let svg_string = qrcode.render::<svg::Color>().build();
    fs::write(path, svg_string)?;
    Ok(())
}

/// 保存登录状态到隐藏文件
fn save_login_state(login_url: &LoginUrl) -> Result<()> {
    let state = serde_json::to_string(login_url)?;
    fs::write(STATE_FILE, state)?;
    Ok(())
}

/// 从隐藏文件读取登录状态
fn load_login_state() -> Result<LoginUrl> {
    let content = fs::read_to_string(STATE_FILE)?;
    let login_url: LoginUrl = serde_json::from_str(&content)?;
    Ok(login_url)
}

/// 生成二维码并保存状态
async fn run_qr(url_only: bool) -> Result<LoginUrl> {
    log::info!("开始获取登录二维码...");

    let login_result = LoginUrl::get_login_url().await?;

    if login_result.code != 0 {
        log::error!("获取登录二维码失败: {:?}", login_result.message);
        return Err(anyhow::anyhow!(
            "获取登录二维码失败: {:?}",
            login_result.message
        ));
    }

    let login_url = login_result
        .data
        .ok_or(anyhow::anyhow!("获取登录二维码失败: 无数据"))?;

    // 显示二维码或只输出链接
    if url_only {
        println!("{}", login_url.url);
    } else {
        display_qrcode(&login_url.url)?;
    }

    // 生成 SVG 二维码文件
    save_qrcode_svg(&login_url.url, "qrcode.svg")?;
    println!("二维码已保存到: qrcode.svg");

    // 保存登录状态
    save_login_state(&login_url)?;
    println!("登录状态已保存到: {}", STATE_FILE);

    Ok(login_url)
}

/// 轮询登录状态
async fn run_poll(login_url: LoginUrl, output: String) -> Result<()> {
    log::info!("等待扫码确认...");

    match login_url.poll_tokens().await {
        Ok(result) => {
            if let Some(client) = result.data {
                println!("\n登录成功!");
                println!("用户ID: {}", client.token.uid);

                let uid = &client.token.uid.to_string();
                let cookies = &client.cookies;

                save_toml_token(&output, uid, cookies)?;
                println!("Token 已保存到: {}", output);

                // 登录成功后删除状态文件
                let _ = fs::remove_file(STATE_FILE);
            } else {
                log::error!("\n登录失败: {:?}", result.message);
                return Err(anyhow::anyhow!("登录失败: {:?}", result.message));
            }
        }
        Err(e) => {
            log::error!("\n登录出错: {}", e);
            return Err(anyhow::anyhow!("登录出错: {}", e));
        }
    }

    Ok(())
}

/// 登录：根据状态文件是否存在自动选择生成二维码或轮询
pub async fn run_login(url_only: bool, output: String) -> Result<()> {
    // 检查状态文件是否存在
    if std::path::Path::new(STATE_FILE).exists() {
        // 状态文件存在，执行轮询
        let login_url = load_login_state()?;
        run_poll(login_url, output).await
    } else {
        // 状态文件不存在，生成二维码
        let _ = run_qr(url_only).await?;
        Ok(())
    }
}
