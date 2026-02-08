use std::{sync::Arc, time::Duration};

use reqwest::{
    cookie::{CookieStore, Jar},
    header::{ACCEPT, ORIGIN, REFERER, USER_AGENT},
    Client,
};
use serde::{Deserialize, Serialize};

mod wbi;

const BILI_URL: &'static str = "https://bilibili.com";

const COOKIE_USER_ID: &'static str = "DedeUserID=";
const COOKIE_SESSDATA: &'static str = "SESSDATA=";
const COOKIE_BILI_JCT: &'static str = "bili_jct=";

const UA: &'static str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/81.0.4044.138 Safari/537.36";

#[derive(Debug, Clone, Default)]
pub struct UserToken {
    pub uid: String,
    pub token: String,
    pub csrf: String,
}

/// # Example
///
///
/// * 在保存了 tokens 的情况下新建一个 `APIClient` 实例
///
/// ```no_run
/// let tokens:Vec<String> = /* 从你保存的地方读回来 */
/// let (token, jar) = UserToken::create_from_tokens(&tokens).unwrap();
/// let client = APIClient::new(token, jar).unwrap();
/// ```
///
/// * 在没有保存 tokens 的情况下，可以通过扫码登录获取 `APIClient`
///
/// ```no_run
/// let login_url = LoginUrl::get_login_url().await.unwrap();
/// let url = login_url.url;
/// /* 把 url 生成一个 qrcode 让用户去扫码确认登录 */
/// let client = login_url.poll_tokens().await.unwrap().data.unwrap();
/// ```
///
#[derive(Debug, Clone)]
pub struct APIClient {
    pub client: Client,
    pub token: UserToken,
    pub jar: Arc<Jar>,
    pub cookies: Vec<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum CheckCookieError {
    #[error("Empty cookie")]
    EmptyCookie,
    #[error("Illegal cookie")]
    IllegalCookie,
    #[error("cookie error {0}")]
    CookieToStrError(#[from] reqwest::header::ToStrError),
}

impl UserToken {
    pub fn create_from_tokens<S: AsRef<str>>(
        tokens: &[S],
    ) -> Result<(Self, Arc<Jar>), CheckCookieError> {
        let domain_url = BILI_URL.parse().unwrap();
        let jar = Arc::new(Jar::default());
        for cookie in tokens {
            jar.add_cookie_str(cookie.as_ref(), &domain_url);
        }
        Ok((Self::create_from_jar(jar.clone())?, jar))
    }

    pub fn create_from_jar(jar: Arc<Jar>) -> Result<Self, CheckCookieError> {
        let domain_url = BILI_URL.parse().unwrap();
        let cookies = jar
            .cookies(&domain_url)
            .ok_or(CheckCookieError::EmptyCookie)?;

        let cookies = cookies.to_str()?;
        let mut token = UserToken::default();

        for c in cookies.split(";") {
            let c = c.trim();
            if c.starts_with(COOKIE_USER_ID) {
                let (_, v) = c.split_at(COOKIE_USER_ID.len());
                token.uid = v.to_string();
            } else if c.starts_with(COOKIE_SESSDATA) {
                let (_, v) = c.split_at(COOKIE_SESSDATA.len());
                token.token = v.to_string();
            } else if c.starts_with(COOKIE_BILI_JCT) {
                let (_, v) = c.split_at(COOKIE_BILI_JCT.len());
                token.csrf = v.to_string();
            } else {
                log::debug!("read cookie: {}", c)
            }
        }

        if token.uid.is_empty() || token.token.is_empty() || token.csrf.is_empty() {
            Err(CheckCookieError::IllegalCookie)
        } else {
            Ok(token)
        }
    }
}

impl APIClient {
    pub fn new(
        token: UserToken,
        jar: Arc<Jar>,
        cookies: Vec<String>,
    ) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .cookie_provider(jar.clone())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self {
            client,
            token,
            jar,
            cookies,
        })
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct QrResult {
    url: String,
    refresh_token: String,
    timestamp: u64,
    // 86101: 未扫码
    // 86038: 二维码已失效
    // 86090：二维码已扫码未确认
    // 0: 确认登录
    code: i32,
    message: String,
}

#[derive(thiserror::Error, Debug)]
pub enum QrResultError {
    /// 86101: 未扫码
    #[error("NotScaned")]
    NotScaned,
    /// 86038: 二维码已失效
    #[error("QrExpired")]
    QrExpired,
    /// 86090：二维码已扫码未确认
    #[error("ScanedNotConfirm")]
    ScanedNotConfirm,
    #[error("UnknownError code: {code}, message: {message}")]
    UnknownError { code: i32, message: String },
    #[error("HttpError {0}")]
    HttpError(#[from] reqwest::Error),
}

impl Into<Result<QrResult, QrResultError>> for QrResult {
    fn into(self) -> Result<QrResult, QrResultError> {
        match self.code {
            86101 => Err(QrResultError::NotScaned),
            86038 => Err(QrResultError::QrExpired),
            86090 => Err(QrResultError::ScanedNotConfirm),
            0 => Ok(self),
            _ => Err(QrResultError::UnknownError {
                code: self.code,
                message: self.message,
            }),
        }
    }
}

async fn check_qrcode(
    client: &Client,
    qrcode_key: &str,
) -> Result<(APIResult<QrResult>, Vec<String>), reqwest::Error> {
    log::info!("get_bili_client by {}", qrcode_key);
    let form_param = [("qrcode_key", qrcode_key), ("source", "main-fe-header")];
    let resp = client
        .get(format!("https://passport.bilibili.com/x/passport-login/web/qrcode/poll?qrcode_key={}&source=main-fe-header", qrcode_key))
        .header(USER_AGENT, UA)
        .header(ACCEPT, "application/json, text/plain, */*")
        .header(REFERER, "https://www.bilibili.com")
        .header(ORIGIN, "https://www.bilibili.com")
        .form(&form_param)
        .send()
        .await?;

    let header_cookies = resp.headers().get_all("set-cookie");
    let mut cookies = Vec::new();

    for cookie_value in header_cookies {
        match cookie_value.to_str() {
            Ok(cookie) => {
                cookies.push(cookie.to_string());
            }
            Err(e) => {
                log::warn!("login cookie to str error : {:?}", e)
            }
        }
    }

    Ok((resp.json::<APIResult<QrResult>>().await?, cookies))
}

async fn poll_tokens_from_bili(
    client: &Client,
    login_url: &LoginUrl,
) -> Result<(APIResult<QrResult>, Vec<String>), QrResultError> {
    'check: loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let (
            APIResult {
                code,
                message,
                ttl,
                ts,
                data,
            },
            cookies,
        ) = check_qrcode(client, &login_url.qrcode_key).await?;

        if code == 0 {
            if let Some(r) = data {
                let r: Result<QrResult, QrResultError> = r.into();
                match r {
                    Ok(r) => {
                        log::info!("get_bili_client success");
                        return Ok((
                            APIResult {
                                code,
                                message,
                                ttl,
                                ts,
                                data: Some(r),
                            },
                            cookies,
                        ));
                    }
                    Err(e) => {
                        log::info!("get_bili_client error: {}", e);
                        match e {
                            QrResultError::NotScaned => {
                                continue 'check;
                            }
                            QrResultError::ScanedNotConfirm => {
                                continue 'check;
                            }
                            e => {
                                return Err(e);
                            }
                        }
                    }
                }
            }
        } else {
            return Ok((
                APIResult {
                    code,
                    message,
                    ttl,
                    ts,
                    data: None,
                },
                cookies,
            ));
        }
    }
}

// api

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct APIResult<T> {
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub ttl: u32,
    #[serde(default)]
    pub ts: u32,
    pub data: Option<T>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LoginUrl {
    pub url: String,
    pub qrcode_key: String,
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("QrResultError {0}")]
    QrResultError(#[from] QrResultError),
    #[error("HttpError {0}")]
    HttpError(#[from] reqwest::Error),
}

impl LoginUrl {
    pub async fn get_login_url() -> Result<APIResult<Self>, reqwest::Error> {
        // https://passport.bilibili.com/x/passport-login/web/qrcode/generate?source=main-fe-header
        let resp = reqwest::get(
            "https://passport.bilibili.com/x/passport-login/web/qrcode/generate?source=main-fe-header",
        )
        .await?;
        resp.json::<APIResult<LoginUrl>>().await
    }

    pub async fn poll_tokens(&self) -> Result<APIResult<APIClient>, LoginError> {
        let jar = Arc::new(Jar::default());

        let client = Client::builder()
            .cookie_provider(jar.clone())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()?;

        // 访问一下 bilibili.com，触发 cookie 的生成
        let _resp = client
            .head("https://www.bilibili.com/")
            .header(USER_AGENT, UA)
            .header(ACCEPT, "application/json, text/plain, */*")
            .send()
            .await?;

        let (
            APIResult {
                code,
                message,
                ttl,
                ts,
                data,
            },
            cookies,
        ) = poll_tokens_from_bili(&client, self).await?;

        if code != 0 || data.is_none() {
            Ok(APIResult {
                code,
                message,
                ttl,
                ts,
                data: None,
            })
        } else {
            let token = UserToken::create_from_jar(jar.clone()).unwrap();
            let client = APIClient::new(token, jar, cookies)?;
            Ok(APIResult {
                code,
                message,
                ttl,
                ts,
                data: Some(client),
            })
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Gift {
    pub gift_id: u64,
    pub price: u64,
}

impl Gift {
    pub const 人气票: Self = Self {
        gift_id: 33988,
        price: 100,
    };
    pub const 喜庆爆竹: Self = Self {
        gift_id: 31569,
        price: 100,
    };

    pub const 贴贴: Self = Self {
        gift_id: 35430,
        price: 1000,
    };

    pub const 做我的小猫: Self = Self {
        gift_id: 34296,
        price: 9900,
    };
}

impl APIClient {
    pub async fn send_barrage(
        &self,
        room_id: &str,
        barrage: &str,
    ) -> Result<APIResult<serde_json::Value>, reqwest::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards");
        let now = format!("{}", now.as_secs());
        let param = [
            ("color", "16777215"), // 默认白色
            ("fontsize", "25"),
            ("mode", "1"), // 1 是滚动弹幕 4 是底部弹幕
            ("msg", barrage),
            ("rnd", now.as_str()),
            ("roomid", room_id),
            ("bubble", "0"),
            ("csrf_token", self.token.csrf.as_str()),
            ("csrf", self.token.csrf.as_str()),
        ];
        let resp = self
            .client
            .post("https://api.live.bilibili.com/msg/send")
            .header(USER_AGENT, UA)
            .header(reqwest::header::REFERER, "https://live.bilibili.com")
            .form(&param)
            .send()
            .await?;

        resp.json::<APIResult<serde_json::Value>>().await
    }

    /// 点赞直播间
    pub async fn like_report_v3(
        &self,
        room_id: &str,
        anchor_id: &str,
        click_time: &str,
    ) -> Result<APIResult<serde_json::Value>, reqwest::Error> {
        let k = wbi::get_wbi_keys().await?;

        let param = [
            ("click_time", click_time.to_string()),
            ("room_id", room_id.to_string()),
            ("uid", self.token.uid.to_string()),
            ("anchor_id", anchor_id.to_string()),
            ("csrf", self.token.csrf.to_string()),
        ];

        let param = wbi::encode_wbi(param.to_vec(), k);
        let url= format!("https://api.live.bilibili.com/xlive/app-ucenter/v1/like_info_v3/like/likeReportV3?{param}");

        let resp = self
            .client
            .post(url)
            .header(USER_AGENT, UA)
            .header(reqwest::header::REFERER, "https://live.bilibili.com")
            .send()
            .await?;

        resp.json::<APIResult<serde_json::Value>>().await
    }

    pub async fn send_gift(
        &self,
        room_id: &str,
        ruid: &str,
        gift: Gift,
        gift_num: u64,
    ) -> Result<APIResult<serde_json::Value>, reqwest::Error> {
        let k = wbi::get_wbi_keys().await?;

        let param = [
            ("uid", self.token.uid.to_string()),
            ("gift_id", gift.gift_id.to_string()),
            ("ruid", ruid.to_string()),
            ("send_ruid", "0".to_string()),
            ("gift_num", gift_num.to_string()),
            ("coin_type", "gold".to_string()),
            ("bag_id", "0".to_string()),
            ("platform", "pc".to_string()),
            ("biz_code", "Live".to_string()),
            ("biz_id", room_id.to_string()),
            ("storm_beat_id", "0".to_string()),
            ("metadata", "".to_string()),
            ("price", gift.price.to_string()),
            ("receive_users", "".to_string()),
            ("live_statistics", "{\"pc_client\":\"pcWeb\",\"jumpfrom\":\"72001\",\"room_category\":\"0\",\"source_event\":0,\"official_channel\":{\"program_room_id\":\"-99998\",\"program_up_id\":\"-99998\"}}".to_string()),
            ("statistics", "{\"platform\":5,\"pc_client\":\"pcWeb\",\"appId\":100}".to_string()),
            ("csrf", self.token.csrf.to_string())];

        let param = wbi::encode_wbi(param.to_vec(), k);
        let url = format!("https://api.live.bilibili.com/xlive/revenue/v1/gift/sendGold?{param}");

        let resp = self
            .client
            .post(url)
            .header(USER_AGENT, UA)
            .header(reqwest::header::REFERER, "https://live.bilibili.com")
            .send()
            .await?;

        resp.json::<APIResult<serde_json::Value>>().await
    }

    /// 分享直播间
    pub async fn share_room(
        &self,
        room_id: &str,
    ) -> Result<APIResult<serde_json::Value>, reqwest::Error> {
        // let k = wbi::get_wbi_keys().await?;

        let param = [
            ("roomid", room_id.to_string()),
            ("interact_type", "3".to_string()),
            ("uid", self.token.uid.to_string()),
            ("csrf", self.token.csrf.to_string()),
            ("csrf_token", self.token.csrf.to_string()),
            ("visit_id", "".to_string()),
        ];

        // let param = wbi::encode_wbi(param.to_vec(), k);
        let url = format!("https://api.live.bilibili.com/xlive/web-room/v1/index/TrigerInteract");

        let resp = self
            .client
            .post(url)
            .header(USER_AGENT, UA)
            .header(
                reqwest::header::REFERER,
                format!("https://live.bilibili.com/{room_id}"),
            )
            .form(&param)
            .send()
            .await?;

        resp.json::<APIResult<serde_json::Value>>().await
    }
}

#[tokio::test]
//  cargo test --package bilili_rs --lib -- api::test_share_room --exact --nocapture
async fn test_share_room() {
    let tokens = std::fs::read_to_string("token").unwrap();
    let tokens: Vec<String> = tokens.split('\n').map(|s| s.to_string()).collect();
    match UserToken::create_from_tokens(&tokens) {
        Ok((token, jar)) => match APIClient::new(token, jar, tokens) {
            Ok(client) => {
                let r = client.share_room("30193402").await;
                println!("r:{r:?}");
                let r = client
                    .like_report_v3("30193402", "3494370399488563", "10")
                    .await;
                println!("r:{r:?}");
            }
            Err(e) => {
                println!("create api client failed: {}", e);
            }
        },
        Err(e) => {
            println!("create api client from tokens failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_send_gift() {
    let tokens = std::fs::read_to_string("token").unwrap();
    let tokens: Vec<String> = tokens.split('\n').map(|s| s.to_string()).collect();
    match UserToken::create_from_tokens(&tokens) {
        Ok((token, jar)) => match APIClient::new(token, jar, tokens) {
            Ok(client) => {
                let r = client
                    .send_gift("1804464760", "1512598845", Gift::人气票, 1)
                    .await;
                println!("r:{r:?}");
                let r = client
                    .like_report_v3("1804464760", "1512598845", "10")
                    .await;
                println!("r:{r:?}");
            }
            Err(e) => {
                println!("create api client failed: {}", e);
            }
        },
        Err(e) => {
            println!("create api client from tokens failed: {}", e);
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DanmuInfoResult {
    #[serde(default)]
    pub business_id: u32,
    #[serde(default)]
    pub host_list: Vec<LiveHost>,
    #[serde(default)]
    pub max_delay: u32,
    #[serde(default)]
    pub refresh_rate: u32,
    #[serde(default)]
    pub refresh_row_factor: f32,
    #[serde(default)]
    pub token: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LiveHost {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u32,
    #[serde(default)]
    pub ws_port: u32,
    #[serde(default)]
    pub wss_port: u32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RoomPlayInfo {
    #[serde(default)]
    pub room_id: u64,
    #[serde(default)]
    pub uid: u64,
    /// 0 关播, 1 直播, 2 轮播
    #[serde(default)]
    pub live_status: u32,
    #[serde(default)]
    pub is_hidden: bool,
    #[serde(default)]
    pub is_locked: bool,
}

#[derive(Deserialize, Debug)]
pub struct LiveRoom {
    pub roomid: u64,
    pub title: String,
    /// 0 关播, 1 直播, 2 轮播
    #[serde(rename = "liveStatus")]
    pub live_status: u32,
    #[serde(rename = "roomStatus")]
    pub room_status: u32,
}

#[derive(Deserialize, Debug)]
pub struct UserInfo {
    pub mid: u64,
    pub name: String,
    pub sex: String,
    #[serde(default)]
    pub live_room: Option<LiveRoom>,
}

impl APIClient {
    /// 获取弹幕服务器信息
    pub async fn get_danmu_info(
        &self,
        room_id: u64,
    ) -> Result<APIResult<DanmuInfoResult>, reqwest::Error> {
        let k = wbi::get_wbi_keys().await?;

        let param = vec![("id", room_id.to_string()), ("type", "0".to_string())];
        let param = wbi::encode_wbi(param, k);
        let resp = self
            .client
            .get(format!(
                "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?{param}",
            ))
            .header(USER_AGENT, UA)
            .send()
            .await?;

        resp.json::<APIResult<DanmuInfoResult>>().await
    }

    /// 获取直播间信息
    pub async fn get_room_play_info(
        &self,
        room_id: u64,
    ) -> Result<APIResult<RoomPlayInfo>, reqwest::Error> {
        let resp = self
            .client
            .get(format!(
                "https://api.live.bilibili.com/xlive/web-room/v2/index/getRoomPlayInfo?room_id={room_id}&protocol=0,1&format=0,1,2&codec=0,1,2&qn=0&platform=web&ptype=8&dolby=5&panorama=1"
            ))
            .header(USER_AGENT, UA)
            .send()
            .await?;

        resp.json::<APIResult<RoomPlayInfo>>().await
    }

    // 获取用户信息
    pub async fn get_user_info(&self, mid: u64) -> Result<APIResult<UserInfo>, reqwest::Error> {
        let k = wbi::get_wbi_keys().await?;

        let param = vec![("platform", "web".to_string()), ("mid", mid.to_string())];
        let param = wbi::encode_wbi(param, k);

        let resp = self
            .client
            .get(format!(
                "https://api.bilibili.com/x/space/wbi/acc/info?{param}"
            ))
            .header(USER_AGENT, UA)
            .send()
            .await?;

        resp.json::<APIResult<UserInfo>>().await
    }
}

#[tokio::test]
async fn test_get_user_info() {
    let tokens = std::fs::read_to_string("token").unwrap();
    let tokens: Vec<String> = tokens.split('\n').map(|s| s.to_string()).collect();
    match UserToken::create_from_tokens(&tokens) {
        Ok((token, jar)) => match APIClient::new(token, jar, tokens) {
            Ok(client) => {
                let r = client.get_user_info(3494370399488563).await;
                println!("r:{r:?}");
            }
            Err(e) => {
                println!("create api client failed: {}", e);
            }
        },
        Err(e) => {
            println!("create api client from tokens failed: {}", e);
        }
    }
}

///
/// # Example
/// ```no_run
/// let login_manager = LoginManager::create(3);
/// let (login_url,api_client_rx) = login_manager.get_one_login_url().await.unwrap();
/// // 让用户扫描 login_url.url 的二维码
/// let api_client = api_client_rx.recv().await.unwrap();
/// // save(client.cookies.join("\n"))
/// ```

#[derive(Debug, Clone)]
pub struct LoginManager {
    login_url_tx: tokio::sync::mpsc::Sender<LoginUrlTx>,
}

#[derive(thiserror::Error, Debug)]
pub enum LoginManagerError {
    #[error("poll thread is stop")]
    PollLoginStop,
    #[error("get login url error: {0}")]
    GetLoginUrlError(String),
    #[error("login url channel closed")]
    LoginUrlTxClosed,
    #[error("login url is timeout")]
    LoginUrlTimeout,
    #[error("login failed: {0:?}")]
    LoginFailed(#[from] LoginError),
}

pub struct APIClientRecv {
    rx: tokio::sync::broadcast::Receiver<Arc<APIClient>>,
}

impl APIClientRecv {
    pub async fn recv(&mut self) -> Result<Arc<APIClient>, LoginManagerError> {
        self.rx
            .recv()
            .await
            .map_err(|_| LoginManagerError::LoginUrlTimeout)
    }
}

type APIClientRx = tokio::sync::broadcast::Receiver<Arc<APIClient>>;
type LoginUrlTx = tokio::sync::oneshot::Sender<Result<(LoginUrl, APIClientRx), LoginManagerError>>;

impl LoginManager {
    pub fn create(retry_times: usize) -> Self {
        let (login_url_tx, login_url_rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(Self::poll_login(retry_times, login_url_rx));
        Self { login_url_tx }
    }

    pub async fn get_one_login_url(&self) -> Result<(LoginUrl, APIClientRecv), LoginManagerError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.login_url_tx
            .send(tx)
            .await
            .map_err(|_| LoginManagerError::PollLoginStop)?;
        let (login_url, api_client_rx) = rx
            .await
            .map_err(|_| LoginManagerError::LoginUrlTxClosed)??;
        Ok((login_url, APIClientRecv { rx: api_client_rx }))
    }

    async fn poll_login(retry_times: usize, mut rx: tokio::sync::mpsc::Receiver<LoginUrlTx>) {
        while let Some(tx) = rx.recv().await {
            // 获取一个新的 login_url
            match Self::get_login_url(retry_times).await {
                Ok(login_url) => {
                    let (api_client_tx, api_client_rx) = tokio::sync::broadcast::channel(1);

                    if tx.send(Ok((login_url.clone(), api_client_rx))).is_ok() {
                        let login_url_ = login_url.clone();

                        // 等待 login_url 的扫描结果
                        let recv_api_client =
                            tokio::spawn(async move { login_url_.poll_tokens().await });

                        // 如果有新的 login_url 请求，就直接发送已经获取到的 login_url 和 现在的 api_client_rx
                        let recv_login_url = async {
                            while let Some(tx) = rx.recv().await {
                                let api_client_rx = api_client_tx.subscribe();
                                let _ = tx.send(Ok((login_url.clone(), api_client_rx)));
                            }
                        };

                        let client = tokio::select! {
                            _ = recv_login_url => {
                                // 到这基本上是因为 login_url_tx 被关闭了，没救了
                                continue
                            }
                            client = recv_api_client => {client.unwrap()},
                        };

                        match client {
                            Ok(client) => {
                                if let Some(api_client) = client.data {
                                    // 广播 token 给所有等待 login_url 的 rx
                                    let _ = api_client_tx.send(Arc::new(api_client));
                                } else {
                                    log::error!(
                                        "poll tokens failed: {}",
                                        client.message.unwrap_or_default()
                                    )
                                }
                            }
                            Err(e) => {
                                log::error!("poll tokens failed: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                    continue;
                }
            }
        }
        log::warn!("login url channel closed");
    }

    async fn get_login_url(retry_times: usize) -> Result<LoginUrl, LoginManagerError> {
        let mut result = String::new();
        for _ in 0..retry_times {
            let login_url = LoginUrl::get_login_url().await;
            return match login_url {
                Ok(login_url) => {
                    if let Some(url) = login_url.data {
                        Ok(url)
                    } else {
                        Err(LoginManagerError::GetLoginUrlError(
                            login_url.message.unwrap_or_default(),
                        ))
                    }
                }
                Err(e) => {
                    result = e.to_string();
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
            };
        }
        Err(LoginManagerError::GetLoginUrlError(result))
    }
}
