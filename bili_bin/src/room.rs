use anyhow::Result;
use bilili_rs::api::RoomPlayInfo;
use crate::client::load_client;

/// 格式化直播间信息
fn format_room_info(info: &RoomPlayInfo) -> String {
    let status = match info.live_status {
        0 => "未开播",
        1 => "直播中",
        2 => "轮播中",
        _ => "未知",
    };

    format!(
        "直播间信息:\n  房间号: {}\n  主播UID: {}\n  状态: {}\n  隐藏: {}\n  锁定: {}",
        info.room_id, info.uid, status, info.is_hidden, info.is_locked
    )
}

/// 获取直播间信息
pub async fn run_room_info(room_id: u64, token_file: String) -> Result<()> {
    log::info!("正在获取直播间 {} 信息...", room_id);

    let client = load_client(&token_file)?;

    match client.get_room_play_info(room_id).await {
        Ok(result) => {
            if result.code == 0 {
                if let Some(info) = result.data {
                    println!("{}", format_room_info(&info));
                } else {
                    return Err(anyhow::anyhow!("未获取到直播间信息"));
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
