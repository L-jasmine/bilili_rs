# bilili_rs

Bilibili 直播间 Rust SDK 和命令行工具。

## 功能特性

- **WebSocket 连接**：实时接收直播间弹幕、礼物等信息
- **HTTP API**：发送弹幕、送礼物、点赞、分享等操作
- **二维码登录**：支持扫码登录，自动管理 Cookie
- **多账号管理**：TOML 格式存储多个账号，支持批量操作
- **设备指纹**：自动获取和管理设备指纹 cookies (buvid3/buvid4)

## 安装

### 预编译二进制

从 [Releases](https://github.com/cshum/bilili_rs/releases) 下载对应平台的二进制文件：

```bash
# Linux/macOS
chmod +x bili_bin
./bili_bin --help

# Windows
bili_bin.exe --help
```

### 从源码构建

```bash
cargo build --release
# 二进制文件位于 target/release/bili_bin
```

## 快速开始

### 1. 登录获取 Token

```bash
# 步骤 1: 生成二维码
bili_bin login -o tokens.toml

# 步骤 2: 用哔哩哔哩 App 扫码后，再次执行相同命令
bili_bin login -o tokens.toml
```

Token 会以 TOML 格式保存到 `tokens.toml`，包含设备指纹信息。

### 2. 使用环境变量（推荐）

```bash
export BILI_TOKEN_FILE=tokens.toml
```

### 3. 发送弹幕

```bash
# 单个账号
bili_bin barrage <房间号> "你好" --uid 123456

# 所有账号
bili_bin barrage <房间号> "你好"
```

## 常用命令

| 命令 | 说明 |
|------|------|
| `login -o <file>` | 二维码登录 |
| `refresh-token` | 刷新设备指纹 |
| `refresh-username` | 刷新用户名 |
| `connect <房间号>` | 连接直播间接收实时消息 |
| `barrage <房间号> <内容>` | 发送弹幕 |
| `gift <房间号> <主播UID> <礼物> <数量>` | 送礼物 |
| `like <房间号> <主播ID> <次数>` | 点赞 |
| `share <房间号>` | 分享直播间 |
| `room <房间号>` | 获取房间信息 |
| `user <用户UID>` | 获取用户信息 |
| `install-skill` | 安装 Claude Code skill |

### 连接直播间

```bash
# 连接并接收实时消息
bili_bin connect <房间号> --uid 123456

# 所有账号连接，只打印一个的消息
bili_bin connect <房间号>

# JSON 模式，配合 jq 过滤
bili_bin connect <房间号> --json | jq 'select(.cmd == "DANMU_MSG")'
```

## Token 文件格式

Token 文件使用 TOML 格式，支持多用户：

```toml
[123456789]
token = """
buvid3=...; Path=/; Domain=.bilibili.com; Max-Age=2147483647
buvid4=...; Path=/; Domain=.bilibili.com; Max-Age=2147483647
SESSDATA=...; Path=/; Domain=bilibili.com; Expires=...
bili_jct=...; Path=/; Domain=bilibili.com; Expires=...
DedeUserID=123456789; Path=/; Domain=bilibili.com; Expires=...
"""
username = "用户昵称"
deadline = "2026-07-09T10:41:07+08:00"
```

`username` 为可选字段，登录时自动获取。

## 批量操作

大部分命令支持两种模式：

1. **指定 `--uid`**：只对指定用户执行操作
2. **不指定 `--uid`**：对所有 token 执行操作（批量模式）

```bash
# 只用 uid=123456 的账号点赞
bili_bin like 8765806 531251 10 --uid 123456

# 用所有账号点赞（批量模式）
bili_bin like 8765806 531251 10
```

## 通用参数

所有命令都支持 `--token-file` / `-t` 参数指定 token 文件，默认为 `token.toml`：

```bash
# 使用环境变量（推荐）
export BILI_TOKEN_FILE=tokens.toml
bili_bin barrage 123456 "hello"

# 或每次指定
bili_bin barrage 123456 "hello" -t tokens.toml
```

## SDK 使用

本项目也提供 Rust SDK：

```toml
[dependencies]
bilili_rs = "0.2"
```

```rust
use bilili_rs::api::APIClient;

// 创建客户端
let client = APIClient::new(token, cookie_jar, tokens)?;

// 发送弹幕
client.send_barrage(room_id, "hello").await?;

// WebSocket 连接接收消息
let mut stream = bilili_rs::live_ws::connect(room_id).await?;
while let Some(msg) = stream.next().await {
    println!("{:?}", msg);
}
```

## 更多文档

详细使用说明请参考 [SKILL.md](.claude/skills/bilili-skill/SKILL.md)

## License

MIT
