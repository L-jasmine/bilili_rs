use anyhow::Result;
use crate::client::{load_all_clients, load_client, ClientWithUid};

/// 给直播间点赞
pub async fn run_like(
    room_id: String,
    anchor_id: String,
    click_count: u64,
    token_file: String,
    uid: Option<&str>,
) -> Result<()> {
    let clients = if uid.is_some() {
        vec![ClientWithUid {
            uid: uid.unwrap().to_string(),
            client: load_client(&token_file, uid)?,
        }]
    } else {
        load_all_clients(&token_file)?
    };

    log::info!(
        "正在给直播间 {} 点赞（点击次数: {}），共 {} 个账号...",
        room_id, click_count, clients.len()
    );

    let mut success_count = 0;
    let mut fail_count = 0;

    for client_with_uid in clients {
        match client_with_uid
            .client
            .like_report_v3(&room_id, &anchor_id, &click_count.to_string())
            .await
        {
            Ok(result) => {
                if result.code == 0 {
                    println!("[uid={}] 点赞成功!", client_with_uid.uid);
                    success_count += 1;
                } else {
                    log::error!("[uid={}] 点赞失败: {:?}", client_with_uid.uid, result.message);
                    fail_count += 1;
                }
            }
            Err(e) => {
                log::error!("[uid={}] 点赞出错: {}", client_with_uid.uid, e);
                fail_count += 1;
            }
        }
    }

    println!("\n点赞完成: 成功 {}, 失败 {}", success_count, fail_count);
    Ok(())
}
