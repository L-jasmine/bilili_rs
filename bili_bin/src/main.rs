mod barrage;
mod client;
mod gift;
mod like;
mod login;
mod room;
mod share;
mod user;

use clap::{Parser, Subcommand};

/// Bilibili 命令行工具
#[derive(Parser)]
#[command(name = "bili")]
#[command(about = "Bilibili 命令行工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 登录（自动检测状态：首次生成二维码，后续轮询登录结果）
    Login {
        /// 只输出二维码链接，不显示二维码图形
        #[arg(short, long)]
        url_only: bool,
        /// 登录成功后保存 cookies 到指定文件
        #[arg(short, long, env = "BILI_TOKEN_FILE")]
        output: String,
    },
    /// 发送弹幕
    Barrage {
        /// 直播间号
        room_id: String,
        /// 弹幕内容
        message: String,
        /// Token 文件路径
        #[arg(short, long, env = "BILI_TOKEN_FILE", default_value = "token")]
        token_file: String,
    },
    /// 分享直播间
    Share {
        /// 直播间号
        room_id: String,
        /// Token 文件路径
        #[arg(short, long, env = "BILI_TOKEN_FILE", default_value = "token")]
        token_file: String,
    },
    /// 给直播间点赞
    Like {
        /// 直播间号
        room_id: String,
        /// 主播 ID
        anchor_id: String,
        /// 点击次数
        click_count: u64,
        /// Token 文件路径
        #[arg(short, long, env = "BILI_TOKEN_FILE", default_value = "token")]
        token_file: String,
    },
    /// 送礼物
    Gift {
        /// 直播间号
        room_id: String,
        /// 主播 UID
        ruid: String,
        /// 礼物名称 (人气票, 喜庆爆竹, 贴贴, 做我的小猫)
        gift_name: String,
        /// 礼物数量
        gift_num: u64,
        /// Token 文件路径
        #[arg(short, long, env = "BILI_TOKEN_FILE", default_value = "token")]
        token_file: String,
    },
    /// 获取直播间信息
    Room {
        /// 直播间号
        room_id: u64,
        /// Token 文件路径
        #[arg(short, long, env = "BILI_TOKEN_FILE", default_value = "token")]
        token_file: String,
    },
    /// 获取用户信息
    User {
        /// 用户 UID (mid)
        mid: u64,
        /// Token 文件路径
        #[arg(short, long, env = "BILI_TOKEN_FILE", default_value = "token")]
        token_file: String,
    },
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let cli = Cli::parse();

    let r = match cli.command {
        Commands::Login { url_only, output } => login::run_login(url_only, output).await,
        Commands::Barrage {
            room_id,
            message,
            token_file,
        } => barrage::run_barrage(room_id, message, token_file).await,
        Commands::Share {
            room_id,
            token_file,
        } => share::run_share(room_id, token_file).await,
        Commands::Like {
            room_id,
            anchor_id,
            click_count,
            token_file,
        } => like::run_like(room_id, anchor_id, click_count, token_file).await,
        Commands::Gift {
            room_id,
            ruid,
            gift_name,
            gift_num,
            token_file,
        } => gift::run_gift(room_id, ruid, gift_name, gift_num, token_file).await,
        Commands::Room {
            room_id,
            token_file,
        } => room::run_room_info(room_id, token_file).await,
        Commands::User { mid, token_file } => user::run_user_info(mid, token_file).await,
    };

    if let Err(e) = r {
        log::error!("Error: {}", e);
        std::process::exit(1);
    }
}
