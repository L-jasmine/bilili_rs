use anyhow::Result;
use bilili_rs::api::Gift;
use crate::client::{load_all_clients, load_client, ClientWithUid};

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
    uid: Option<&str>,
) -> Result<()> {
    let gift = parse_gift(&gift_name)?;

    let clients = if uid.is_some() {
        vec![ClientWithUid {
            uid: uid.unwrap().to_string(),
            client: load_client(&token_file, uid)?,
        }]
    } else {
        load_all_clients(&token_file)?
    };

    log::info!(
        "正在向直播间 {} 送礼物（{} 个 {}），共 {} 个账号...",
        room_id, gift_num, gift_name, clients.len()
    );

    let mut success_count = 0;
    let mut fail_count = 0;

    for client_with_uid in clients {
        match client_with_uid
            .client
            .send_gift(&room_id, &ruid, gift, gift_num)
            .await
        {
            Ok(result) => {
                if result.code == 0 {
                    println!(
                        "[uid={}] 送礼物成功! 送出 {} 个 {}",
                        client_with_uid.uid, gift_num, gift_name
                    );
                    success_count += 1;
                } else {
                    log::error!("[uid={}] 送礼物失败: {:?}", client_with_uid.uid, result.message);
                    fail_count += 1;
                }
            }
            Err(e) => {
                log::error!("[uid={}] 送礼物出错: {}", client_with_uid.uid, e);
                fail_count += 1;
            }
        }
    }

    println!("\n送礼物完成: 成功 {}, 失败 {}", success_count, fail_count);
    Ok(())
}
