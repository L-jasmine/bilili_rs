use anyhow::Result;
use crate::client::load_client;

/// 给直播间点赞
pub async fn run_like(
    room_id: String,
    anchor_id: String,
    click_count: u64,
    token_file: String,
) -> Result<()> {
    log::info!("正在给直播间 {} 点赞，点击次数: {}...", room_id, click_count);

    let client = load_client(&token_file)?;

    match client
        .like_report_v3(&room_id, &anchor_id, &click_count.to_string())
        .await
    {
        Ok(result) => {
            if result.code == 0 {
                println!("点赞成功!");
            } else {
                log::error!("点赞失败: {:?}", result.message);
                return Err(anyhow::anyhow!("点赞失败: {:?}", result.message));
            }
        }
        Err(e) => {
            log::error!("点赞出错: {}", e);
            return Err(anyhow::anyhow!("点赞出错: {}", e));
        }
    }

    Ok(())
}
