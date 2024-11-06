use std::{sync::Arc, time::Duration};

use reqwest::{
    cookie::{CookieStore, Jar},
    header::{ACCEPT, ORIGIN, REFERER, USER_AGENT},
    Client,
};
use serde::{Deserialize, Serialize};

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

impl APIClient {
    /// 获取弹幕服务器信息
    pub async fn get_danmu_info(
        &self,
        room_id: u64,
    ) -> Result<APIResult<DanmuInfoResult>, reqwest::Error> {
        let resp = self
            .client
            .get(format!(
                "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?id={}&type=0",
                room_id
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
            .await
            ?;

        resp.json::<APIResult<RoomPlayInfo>>().await
    }
}
