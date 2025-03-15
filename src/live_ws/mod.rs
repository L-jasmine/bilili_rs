pub mod message;

use crate::api::{APIClient, APIResult, LiveHost};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
pub use message::notification_msg::NotificationMsg;
pub use message::{ClientLiveMessage, ServerLiveMessage, WsLogin};
use std::collections::LinkedList;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use log::{debug, error, info, warn};

#[derive(Debug)]
pub struct MsgStream {
    pub room_id: u64,
    pub rx: Receiver<ServerLiveMessage>,
    _connect_handler: JoinHandle<Result<(), LiveConnectError>>,
}

type WsStream = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type RsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

// const BILI_CHAT_SERVER_URL: &'static str = "wss://broadcastlv.chat.bilibili.com/sub";

pub fn connect(api_client: Arc<APIClient>, room_id: u64, max_retry: u32) -> MsgStream {
    // let url = BILI_CHAT_SERVER_URL.parse().unwrap();

    info!("[{room_id}] ws start connect");

    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let _connect_handler = tokio::spawn(open_client(api_client, room_id, tx, max_retry));
    MsgStream {
        room_id,
        rx,
        _connect_handler,
    }
}

async fn open_bili_ws(
    room_id: u64,
    sub_urls: &[LiveHost],
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, tokio_tungstenite::tungstenite::Error> {
    let mut err = None;
    for host in sub_urls {
        let url = format!("wss://{}/sub", host.host);
        let connect_r = connect_async(&url).await;
        match connect_r {
            Ok((ws_stream, _)) => return Ok(ws_stream),
            Err(e) => {
                error!("ws connect [{room_id}] to {url} error {:?}", e);
                err = Some(e);
                continue;
            }
        };
    }
    Err(err.unwrap())
}

#[derive(thiserror::Error, Debug)]
pub enum LiveConnectError {
    #[error("TxClose")]
    TxClose,
    #[error("IO: {0}")]
    IoError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("RetryTimeout")]
    RetryTimeout,
}

pub async fn open_client(
    api_client: Arc<APIClient>,
    room_id: u64,
    tx: Sender<ServerLiveMessage>,
    max_retry: u32,
) -> Result<(), LiveConnectError> {
    let uid = api_client.token.uid.parse().unwrap();
    let mut reconnect_time = 0u32;
    'a: loop {
        if reconnect_time >= max_retry {
            error!("reconnect [{room_id}] fail");
            return Err(LiveConnectError::RetryTimeout);
        }
        reconnect_time = reconnect_time + 1;
        let start_time = std::time::SystemTime::now();
        let danmu_info = api_client.get_danmu_info(room_id).await;
        let info = match danmu_info {
            Ok(info) => info,
            Err(e) => {
                error!("get [{room_id}] danmu info {}", e);
                continue 'a;
            }
        };

        let info = if let APIResult {
            code: 0,
            data: Some(info),
            ..
        } = info
        {
            info
        } else {
            error!("get [{room_id}] danmu info {:?}", info);
            continue 'a;
        };

        let ws_login = WsLogin {
            room_id,
            uid,
            key: info.token,
        };

        let ws_stream = open_bili_ws(room_id, &info.host_list).await?;
        let (mut w_stream, mut r_stream) = ws_stream.split();
        let r = tokio::try_join!(
            connect_keep(&mut w_stream, ws_login),
            loop_handle_msg(&mut r_stream, tx.clone())
        );
        info!("ws client close [{room_id}] {:?}", r);
        if let Err(LiveConnectError::TxClose) = r {
            return Err(LiveConnectError::TxClose);
        }
        let now = std::time::SystemTime::now();
        let d = now.duration_since(start_time).unwrap().as_secs();
        if d > (60 * 30) {
            reconnect_time = 0;
        }
        let time = if reconnect_time <= 10 { 10 } else { 300 };
        info!("reconnect [{room_id}] [{reconnect_time}] after {time} secs");
        tokio::time::sleep(Duration::from_secs(time)).await;
        info!("reconnect [{room_id}] start");
    }
}

async fn connect_keep(client: &mut WsStream, ws_login: WsLogin) -> Result<(), LiveConnectError> {
    client
        .send(Message::Binary(ClientLiveMessage::Login(ws_login).encode()))
        .await?;
    loop {
        debug!("heartbeat");
        client
            .send(Message::Binary(ClientLiveMessage::ClientHeartBeat.encode()))
            .await?;
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

async fn loop_handle_msg(
    client: &mut RsStream,
    tx: Sender<ServerLiveMessage>,
) -> Result<(), LiveConnectError> {
    let mut msg_list = LinkedList::new();
    while let Some(msg) = client.next().await {
        let msg = msg?;
        match msg {
            Message::Text(text) => {
                debug!("recv text {}", text)
            }
            Message::Binary(bin) => {
                if let Err(e) = message::decode_from_server(bin, &mut msg_list) {
                    if matches!(e, message::MsgDecodeError::DecodeBodyError(_)) {
                        debug!("handler msg {:?}", e);
                    } else {
                        warn!("handler msg {:?}", e);
                    }
                }
                while let Some(msg) = msg_list.pop_front() {
                    match &msg {
                        ServerLiveMessage::LoginAck => {
                            debug!("LoginAck");
                        }
                        ServerLiveMessage::Notification(_) => {
                            debug!("Notification");
                        }
                        ServerLiveMessage::ServerHeartBeat => {
                            debug!("ServerHeartBeat");
                        }
                    }
                    tx.send(msg).await.map_err(|_| LiveConnectError::TxClose)?;
                    debug!("send msg ok");
                }
            }
            Message::Ping(_) => debug!("ws ping"),
            Message::Pong(_) => debug!("ws pong"),
            Message::Close(_) => {
                warn!("ws close");
                break;
            }
            Message::Frame(_) => warn!("ws frame (unreachable)"),
        }
    }
    warn!("ws handle loop stop");
    Err(LiveConnectError::IoError(
        tokio_tungstenite::tungstenite::Error::ConnectionClosed,
    ))
}
