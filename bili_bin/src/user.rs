use anyhow::Result;
use bilili_rs::api::UserInfo;
use crate::client::load_client;

/// 格式化用户信息
fn format_user_info(info: &UserInfo) -> String {
    let status = match &info.live_room {
        Some(room) => match room.live_status {
            0 => "未开播",
            1 => "直播中",
            2 => "轮播中",
            _ => "未知",
        },
        None => "无直播间",
    };

    let room_info = info.live_room.as_ref().map(|room| {
        format!(
            "\n  直播间号: {}\n  直播标题: {}",
            room.roomid, room.title
        )
    }).unwrap_or_default();

    format!(
        "用户信息:\n  UID: {}\n  昵称: {}\n  性别: {}\n  直播状态: {}{}",
        info.mid, info.name, info.sex, status, room_info
    )
}

/// 获取用户信息
pub async fn run_user_info(mid: u64, token_file: String) -> Result<()> {
    log::info!("正在获取用户 {} 信息...", mid);

    let client = load_client(&token_file)?;

    match client.get_user_info(mid).await {
        Ok(result) => {
            if result.code == 0 {
                if let Some(info) = result.data {
                    println!("{}", format_user_info(&info));
                } else {
                    return Err(anyhow::anyhow!("未获取到用户信息"));
                }
            } else {
                log::error!("获取失败: {:?}", result.message);
                return Err(anyhow::anyhow!("获取失败: {:?}", result.message));
            }
        }
        Err(e) => {
            log::error!("获取出错: {}", e);
            return Err(anyhow::anyhow!("获取出错: {}", e));
        }
    }

    Ok(())
}
