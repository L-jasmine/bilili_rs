use crate::client::{ClientWithUid, load_all_clients, load_client};
use anyhow::Result;

/// 分享直播间
pub async fn run_share(room_id: String, token_file: String, uid: Option<&str>) -> Result<()> {
    let clients = if uid.is_some() {
        vec![ClientWithUid {
            uid: uid.unwrap().to_string(),
            client: load_client(&token_file, uid)?,
        }]
    } else {
        load_all_clients(&token_file)?
    };

    log::info!("正在分享直播间 {}，共 {} 个账号...", room_id, clients.len());

    let mut success_count = 0;
    let mut fail_count = 0;

    for client_with_uid in clients {
        match client_with_uid.client.share_room(&room_id).await {
            Ok(result) => {
                if result.code == 0 {
                    println!("[uid={}] 分享成功!", client_with_uid.uid);
                    success_count += 1;
                } else {
                    log::error!(
                        "[uid={}] 分享失败: {:?}",
                        client_with_uid.uid,
                        result.message
                    );
                    fail_count += 1;
                }
            }
            Err(e) => {
                log::error!("[uid={}] 分享出错: {}", client_with_uid.uid, e);
                fail_count += 1;
            }
        }
    }

    println!("\n分享完成: 成功 {}, 失败 {}", success_count, fail_count);
    Ok(())
}
