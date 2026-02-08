---
name: bilili-skill
description: Bilibili 直播间 CLI 工具。使用此技能执行 Bilibili 直播间操作：登录、发送弹幕、送礼物、点赞、分享、获取房间信息、获取用户信息。
---

## bili_bin - Bilibili 直播间命令行工具

用法：`./scripts/bili_bin <command> [args...]`

Tip: 有时不知道房间号或者只有主播名字时，可以尝试通过 WebSearch 来获取。

### 登录

```bash
# 步骤 1: 生成二维码
./scripts/bili_bin login -o token.txt

# 步骤 2: 用户扫码后，再次执行相同命令完成登录
./scripts/bili_bin login -o token.txt

# 只输出二维码链接（不显示图形）
./scripts/bili_bin login --url-only -o token.txt
```

登录流程：
1. 首次运行生成二维码，保存到 `qrcode.svg` 和 `.bili_login_state`
2. 让用户使用哔哩哔哩手机 App 扫描二维码
3. 用户回答已经扫码后，再次执行 `login` 命令，程序会检测到状态文件并轮询登录状态
4. 登录成功后保存 cookies 到指定文件（如 `token.txt`），并自动删除 `.bili_login_state`

### 发送弹幕

```bash
./scripts/bili_bin barrage <房间号> <弹幕内容>

# 示例
./scripts/bili_bin barrage 123456 "你好直播间"
```

### 分享直播间

```bash
./scripts/bili_bin share <房间号>
```

### 点赞直播间

```bash
./scripts/bili_bin like <房间号> <主播ID> <点击次数>

# 示例
./scripts/bili_bin like 123456 789 "10"
```

### 送礼物

```bash
./scripts/bili_bin gift <房间号> <主播UID> <礼物名称> <数量>

# 可用礼物: 人气票, 喜庆爆竹, 贴贴, 做我的小猫
# 示例
./scripts/bili_bin gift 123456 789 "人气票" 1
./scripts/bili_bin gift 123456 789 "贴贴" 5
```

### 获取房间信息

```bash
./scripts/bili_bin room <房间号>

# 输出示例
# 直播间信息:
#   房间号: 123456
#   主播UID: 789
#   状态: 直播中
#   隐藏: false
#   锁定: false
```

### 获取用户信息

```bash
./scripts/bili_bin user <用户UID>

# 示例
./scripts/bili_bin user 12345

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

所有命令都支持 `--token-file` / `-t` 参数指定 token 文件，默认为 `token`：

```bash
# 使用环境变量（推荐）
export BILI_TOKEN_FILE=token.txt
./scripts/bili_bin barrage 123456 "hello"

# 或每次指定
./scripts/bili_bin barrage 123456 "hello" -t token.txt
```

### 查看帮助

```bash
./scripts/bili_bin --help
./scripts/bili_bin barrage --help
./scripts/bili_bin gift --help
```
