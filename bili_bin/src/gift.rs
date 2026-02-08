use anyhow::Result;
use bilili_rs::api::Gift;
use crate::client::load_client;

/// 礼物名称到 Gift 的映射
fn parse_gift(name: &str) -> Result<Gift> {
    match name {
        "人气票" => Ok(Gift::人气票),
        "喜庆爆竹" => Ok(Gift::喜庆爆竹),
        "贴贴" => Ok(Gift::贴贴),
        "做我的小猫" => Ok(Gift::做我的小猫),
        _ => Err(anyhow::anyhow!(
            "未知礼物: {}。可选: 人气票, 喜庆爆竹, 贴贴, 做我的小猫",
            name
        )),
    }
}

/// 送礼物
pub async fn run_gift(
    room_id: String,
    ruid: String,
    gift_name: String,
    gift_num: u64,
    token_file: String,
) -> Result<()> {
    log::info!("正在向直播间 {} 送礼物...", room_id);

    let client = load_client(&token_file)?;
    let gift = parse_gift(&gift_name)?;

    match client.send_gift(&room_id, &ruid, gift, gift_num).await {
        Ok(result) => {
            if result.code == 0 {
                println!("送礼物成功! 送出 {} 个 {}", gift_num, gift_name);
            } else {
                log::error!("送礼物失败: {:?}", result.message);
                return Err(anyhow::anyhow!("送礼物失败: {:?}", result.message));
            }
        }
        Err(e) => {
            log::error!("送礼物出错: {}", e);
            return Err(anyhow::anyhow!("送礼物出错: {}", e));
        }
    }

    Ok(())
}
