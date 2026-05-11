use anyhow::Result;
use crate::client::{load_all_clients, load_client, ClientWithUid};

/// 发送弹幕
pub async fn run_barrage(
    room_id: String,
    message: String,
    token_file: String,
    uid: Option<&str>,
) -> Result<()> {
    let clients = if uid.is_some() {
        // 指定了 uid，只加载一个
        vec![ClientWithUid {
            uid: uid.unwrap().to_string(),
            client: load_client(&token_file, uid)?,
        }]
    } else {
        // 未指定 uid，加载所有
        load_all_clients(&token_file)?
    };

    log::info!("正在发送弹幕到直播间 {}，共 {} 个账号...", room_id, clients.len());

    let mut success_count = 0;
    let mut fail_count = 0;

    for client_with_uid in clients {
        match client_with_uid
            .client
            .send_barrage(&room_id, &message)
            .await
        {
            Ok(result) => {
                if result.code == 0 {
                    println!("[uid={}] 弹幕发送成功!", client_with_uid.uid);
                    success_count += 1;
                } else {
                    log::error!("[uid={}] 发送失败: {:?}", client_with_uid.uid, result.message);
                    fail_count += 1;
                }
            }
            Err(e) => {
                log::error!("[uid={}] 发送出错: {}", client_with_uid.uid, e);
                fail_count += 1;
            }
        }
    }

    println!("\n发送完成: 成功 {}, 失败 {}", success_count, fail_count);
    Ok(())
}
