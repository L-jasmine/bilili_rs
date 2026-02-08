use anyhow::Result;
use crate::client::load_client;

/// 分享直播间
pub async fn run_share(room_id: String, token_file: String) -> Result<()> {
    log::info!("正在分享直播间 {}...", room_id);

    let client = load_client(&token_file)?;

    match client.share_room(&room_id).await {
        Ok(result) => {
            if result.code == 0 {
                println!("分享成功!");
            } else {
                log::error!("分享失败: {:?}", result.message);
                return Err(anyhow::anyhow!("分享失败: {:?}", result.message));
            }
        }
        Err(e) => {
            log::error!("分享出错: {}", e);
            return Err(anyhow::anyhow!("分享出错: {}", e));
        }
    }

    Ok(())
}
