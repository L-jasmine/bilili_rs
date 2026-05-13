---
name: bilili-skill
description: Bilibili 直播间 CLI 工具。使用此技能执行 Bilibili 直播间操作：登录、刷新 token、刷新用户名、连接直播间、发送弹幕、送礼物、点赞、分享、获取房间信息、获取用户信息、安装 skill。
---

## bili_bin - Bilibili 直播间命令行工具

用法：`bili_bin <command> [args...]`

Tip: 有时不知道房间号或者只有主播名字时，可以尝试通过 WebSearch 来获取。

### 安装

从 GitHub Releases 下载最新版本：

```bash
# 自动检测平台并下载
curl -sL "https://api.github.com/repos/L-jasmine/bilili_rs/releases/latest" | \
  grep "browser_download_url" | \
  grep "$(uname | tr '[:upper:]' '[:lower:]')" | \
  cut -d '"' -f 4 | \
  xargs -n 1 curl -sLO

# 或直接下载指定平台
wget https://github.com/L-jasmine/bilili_rs/releases/latest/download/bili_bin-linux   # Linux
wget https://github.com/L-jasmine/bilili_rs/releases/latest/download/bili_bin-macos   # macOS
wget https://github.com/L-jasmine/bilili_rs/releases/latest/download/bili_bin.exe     # Windows

# 重命名并添加执行权限
mv bili_bin-* bili_bin
chmod +x bili_bin

# Windows 用户重命名为 bili_bin.exe 即可
```

### Token 文件格式

Token 文件使用 **TOML 格式**，支持多用户管理：

```toml
[uid1]
token = """
buvid3=...; Path=/; Domain=.bilibili.com; Max-Age=2147483647
buvid4=...; Path=/; Domain=.bilibili.com; Max-Age=2147483647
SESSDATA=...; Path=/; Domain=bilibili.com; Expires=...
bili_jct=...; Path=/; Domain=bilibili.com; Expires=...
DedeUserID=uid1; Path=/; Domain=bilibili.com; Expires=...
...
"""
username = "用户昵称"
deadline = "2026-07-09T10:41:07.088606900+08:00"

[uid2]
token = """..."""
deadline = "..."
```

`username` 为可选字段，登录时自动获取，也可通过 `refresh-username` 命令手动刷新。

### 登录

```bash
# 步骤 1: 生成二维码（输出文件必须以 .toml 结尾）
bili_bin login -o tokens.toml

# 步骤 2: 用户扫码后，再次执行相同命令完成登录
bili_bin login -o tokens.toml

# 只输出二维码链接（不显示图形）
bili_bin login --url-only -o tokens.toml
```

登录流程：
1. 首次运行生成二维码，保存到 `qrcode.svg` 和 `.bili_login_state`
2. 让用户使用哔哩哔哩手机 App 扫描二维码
3. 用户回答已经扫码后，再次执行 `login` 命令，程序会检测到状态文件并轮询登录状态
4. 登录成功后保存 cookies 到指定的 TOML 文件，自动获取用户名并保存，最后删除 `.bili_login_state`

**注意**：登录成功后，token 会自动包含设备指纹 cookies (buvid3/buvid4)，这些是点赞等操作所必需的。

### 刷新 Token

为现有的 token 文件补充设备指纹 cookies (buvid3/buvid4)，无需重新登录：

```bash
# 刷新所有 token（每个 token 获取独立的 fingerprint）
bili_bin refresh-token -t tokens.toml

# 只刷新指定 uid 的 token
bili_bin refresh-token -t tokens.toml --uid 123456

# 使用环境变量
export BILI_TOKEN_FILE=tokens.toml
bili_bin refresh-token
```

**使用场景**：
- 如果点赞功能返回 -352 错误（风控校验失败），通常是缺少设备指纹 cookies
- 使用老版本登录的 token 文件可以用此命令更新

### 刷新用户名

查询 B 站 API 获取每个 uid 的用户名并保存到 token 文件：

```bash
# 刷新所有 uid 的用户名（每个间隔 500ms）
bili_bin refresh-username -t tokens.toml

# 只刷新指定 uid
bili_bin refresh-username -t tokens.toml --uid 123456
```

### 安装 Skill

将 Claude Code skill 安装到项目中：

```bash
# 安装到当前目录的 .claude/skills/（默认）
bili_bin install-skill --local

# 安装到全局 ~/.claude/skills/（~/.claude 必须存在）
bili_bin install-skill --global
```

### 批量操作与 --uid 参数

支持以下两种模式：

1. **指定 `--uid`**：只对指定用户执行操作
2. **不指定 `--uid`**：对所有 token 执行操作（批量模式）

```bash
# 只用 uid=123456 的账号点赞
bili_bin like 8765806 531251 10 -t tokens.toml --uid 123456

# 用所有账号点赞（批量模式）
bili_bin like 8765806 531251 10 -t tokens.toml
```

**注意**：`user-info` 命令**必须**指定 `--uid`。`room` 命令不指定 `--uid` 时会遍历所有 token 直到成功。

### 发送弹幕

```bash
# 指定 uid 发送
bili_bin barrage <房间号> <弹幕内容> --uid 123456

# 批量发送（所有账号）
bili_bin barrage <房间号> <弹幕内容>
```

### 分享直播间

```bash
# 指定 uid 分享
bili_bin share <房间号> --uid 123456

# 批量分享（所有账号）
bili_bin share <房间号>
```

### 点赞直播间

```bash
# 指定 uid 点赞
bili_bin like <房间号> <主播ID> <点击次数> --uid 123456

# 批量点赞（所有账号）
bili_bin like <房间号> <主播ID> <点击次数>
```

### 送礼物

```bash
# 指定 uid 送礼物
bili_bin gift <房间号> <主播UID> <礼物名称> <数量> --uid 123456

# 批量送礼物（所有账号）
bili_bin gift <房间号> <主播UID> <礼物名称> <数量>

# 可用礼物: 人气票, 喜庆爆竹, 贴贴, 做我的小猫
# 示例
bili_bin gift 123456 789 "人气票" 1
bili_bin gift 123456 789 "贴贴" 5
```

### 连接直播间

连接直播间 WebSocket，实时接收弹幕、礼物、进入等消息：

```bash
# 指定 uid 连接
bili_bin connect <房间号> --uid 123456

# 不指定 uid，所有账号连接但只打印一个的消息流
bili_bin connect <房间号>
```

输出格式：
```
[弹幕] 用户名: 弹幕内容
[礼物] 用户名 送出 礼物名 x数量
[进入] 用户名 进入直播间
[上舰] 用户名 购买了 礼物名 x数量
[直播] 开播了 / 下播了
```

**JSON 模式**：使用 `--json` 输出原始 JSON，便于 pipe 给 `jq` 过滤：

```bash
# 只看弹幕
bili_bin connect <房间号> --json | jq 'select(.cmd == "DANMU_MSG")'

# 只看金额大于1000的礼物
bili_bin connect <房间号> --json | jq 'select(.cmd == "SEND_GIFT" and .data.total_coin > 1000)'

# 只看舰长等级 >= 2 的弹幕
bili_bin connect <房间号> --json | jq 'select(.cmd == "DANMU_MSG" and .info.guard_level >= 2)'
```

### 获取房间信息

```bash
# 指定 uid
bili_bin room <房间号> --uid 123456

# 不指定 uid 时遍历所有 token 直到成功
bili_bin room <房间号>

# 输出示例
# 直播间信息:
#   房间号: 123456
#   主播UID: 789
#   状态: 直播中
#   隐藏: false
#   锁定: false
```

### 获取用户信息

**必须指定 `--uid`**：

```bash
bili_bin user <用户UID> --uid 123456

# 输出示例
# 用户信息:
#   UID: 12345
#   昵称: xxx
#   性别: 男
#   直播状态: 直播中
#   直播间号: 123456
#   直播标题: xxx
```

### 通用参数

所有命令都支持 `--token-file` / `-t` 参数指定 token 文件，默认为 `token.toml`：

```bash
# 使用环境变量（推荐）
export BILI_TOKEN_FILE=tokens.toml
bili_bin barrage 123456 "hello"

# 或每次指定
bili_bin barrage 123456 "hello" -t tokens.toml
```

### 查看帮助

```bash
bili_bin --help
bili_bin barrage --help
bili_bin gift --help
```
