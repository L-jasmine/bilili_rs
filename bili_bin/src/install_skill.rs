use anyhow::Result;
use std::fs;
use std::path::PathBuf;

const SKILL_CONTENT: &str = include_str!("../../.claude/skills/bilili-skill/SKILL.md");

pub fn run_install_skill(global: bool, _local: bool) -> Result<()> {
    let target_dir = if global {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| "/tmp".to_string());
        let claude_dir = PathBuf::from(home).join(".claude");
        if !claude_dir.exists() {
            return Err(anyhow::anyhow!(
                "全局目录 {} 不存在，请先创建或使用 --local 安装到当前项目",
                claude_dir.display()
            ));
        }
        claude_dir.join("skills").join("bilili-skill")
    } else {
        // --local 或默认行为
        std::env::current_dir()?
            .join(".claude")
            .join("skills")
            .join("bilili-skill")
    };

    fs::create_dir_all(&target_dir)?;

    let target_file = target_dir.join("SKILL.md");
    fs::write(&target_file, SKILL_CONTENT)?;

    println!("已安装 skill 到 {}", target_file.display());
    Ok(())
}
