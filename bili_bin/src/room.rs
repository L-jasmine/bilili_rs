use crate::client::{load_all_clients, load_client};
use anyhow::Result;
use bilili_rs::api::RoomPlayInfo;

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
pub async fn run_room_info(room_id: u64, token_file: String, uid: Option<&str>) -> Result<()> {
    if let Some(uid) = uid {
        log::info!("正在获取直播间 {} 信息 (uid: {})...", room_id, uid);
        let client = load_client(&token_file, Some(uid))?;
        fetch_and_print(&client, room_id).await
    } else {
        let clients = load_all_clients(&token_file)?;
        let mut last_err = None;
        for c in &clients {
            log::info!("正在获取直播间 {} 信息 (uid: {})...", room_id, c.uid);
            match fetch_and_print(&c.client, room_id).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    log::warn!("uid={} 获取失败: {}", c.uid, e);
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("没有可用的 token")))
    }
}

async fn fetch_and_print(client: &bilili_rs::api::APIClient, room_id: u64) -> Result<()> {
    let result = client.get_room_play_info(room_id).await?;
    if result.code == 0 {
        if let Some(info) = result.data {
            println!("{}", format_room_info(&info));
            Ok(())
        } else {
            Err(anyhow::anyhow!("未获取到直播间信息"))
        }
    } else {
        Err(anyhow::anyhow!("获取失败: {:?}", result.message))
    }
}
