use anyhow::Result;
use crate::client::load_client;

/// 发送弹幕
pub async fn run_barrage(
    room_id: String,
    message: String,
    token_file: String,
) -> Result<()> {
    log::info!("正在发送弹幕到直播间 {}...", room_id);

    let client = load_client(&token_file)?;

    match client.send_barrage(&room_id, &message).await {
        Ok(result) => {
            if result.code == 0 {
                println!("弹幕发送成功!");
            } else {
                log::error!("发送失败: {:?}", result.message);
                return Err(anyhow::anyhow!("发送失败: {:?}", result.message));
            }
        }
        Err(e) => {
            log::error!("发送出错: {}", e);
            return Err(anyhow::anyhow!("发送出错: {}", e));
        }
    }

    Ok(())
}
