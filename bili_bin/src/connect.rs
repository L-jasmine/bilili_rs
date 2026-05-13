use std::sync::Arc;

use anyhow::Result;
use bilili_rs::live_ws::{self, ServerLiveMessage};

use crate::client::{load_all_clients, load_client};

pub async fn run_connect(
    room_id: u64,
    token_file: String,
    uid: Option<&str>,
    json: bool,
) -> Result<()> {
    if !json {
        println!("正在连接直播间 {}...", room_id);
    }

    if let Some(uid) = uid {
        let client = load_client(&token_file, Some(uid))?;
        let client = Arc::new(client);

        let msg_stream = live_ws::connect(client, room_id, 100);
        let mut rx = msg_stream.rx;

        if !json {
            println!(
                "已连接直播间 {} (uid: {})，等待消息... (Ctrl+C 退出)",
                room_id, uid
            );
        }
        print_messages(&mut rx, json).await;
    } else {
        let clients = load_all_clients(&token_file)?;
        let total = clients.len();

        let msg_stream = live_ws::connect(Arc::new(clients[0].client.clone()), room_id, 100);
        let mut rx = msg_stream.rx;

        for c in &clients[1..] {
            live_ws::connect(Arc::new(c.client.clone()), room_id, 100);
        }

        if !json {
            println!(
                "已连接直播间 {} ({} 个账号)，等待消息... (Ctrl+C 退出)",
                room_id, total
            );
        }
        print_messages(&mut rx, json).await;
    }

    if !json {
        println!("连接已断开");
    }
    Ok(())
}

async fn print_messages(rx: &mut tokio::sync::mpsc::Receiver<ServerLiveMessage>, json: bool) {
    while let Some(msg) = rx.recv().await {
        match msg {
            ServerLiveMessage::LoginAck => {
                if json {
                    println!("{}", serde_json::json!({"cmd": "LOGIN_ACK"}));
                } else {
                    println!("[系统] 连接成功");
                }
            }
            ServerLiveMessage::ServerHeartBeat => {
                log::debug!("heartbeat");
            }
            ServerLiveMessage::Notification(notification) => {
                if json {
                    if let Ok(json_str) = serde_json::to_string(&notification) {
                        println!("{}", json_str);
                    }
                } else {
                    print_notification(notification);
                }
            }
        }
    }
}

fn print_notification(notification: live_ws::notification_msg::NotificationMsg) {
    use live_ws::notification_msg::NotificationMsg::*;

    match notification {
        DANMU_MSG { info } => {
            println!("[弹幕] {}: {}", info.uname, info.text);
        }
        SEND_GIFT { data } => {
            println!(
                "[礼物] {} 送出 {} x{}",
                data.uname, data.gift_name, data.num
            );
        }
        COMBO_SEND { data } => {
            println!(
                "[连击] {} 送出 {} x{}",
                data.uname, data.gift_name, data.total_num
            );
        }
        GUARD_BUY { data } => {
            println!(
                "[上舰] {} 购买了 {} x{}",
                data.username, data.gift_name, data.num
            );
        }
        INTERACT_WORD { data } => {
            println!("[进入] {} 进入直播间", data.uname);
        }
        ENTRY_EFFECT { data } => {
            if !data.copy_writing.is_empty() {
                println!("[特效] {}", data.copy_writing);
            }
        }
        LIVE {} => {
            println!("[直播] 开播了");
        }
        PREPARING { roomid } => {
            println!("[直播] 下播了 (房间: {})", roomid);
        }
        _ => {
            log::debug!("收到未处理的消息类型");
        }
    }
}
