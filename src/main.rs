use clap::{Parser, Subcommand};
use duct::cmd;
use futures::future;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::{env, io, sync::Arc};
use tera::{Context, Tera};
use thiserror::Error;
use tokio::{
    fs::{File, OpenOptions, create_dir, create_dir_all, read_to_string, write},
    io::AsyncWriteExt,
    task,
};
use tracing::{error, info};
use tracing_subscriber::fmt;
use walkdir::WalkDir;

//插件模板文件仓库地址
const OHOS_DIRECTORY_GIT_URL: &str = "https://github.com/mdddj/flutter_rust_plugin_ohos_temp";

/// Flutter Rust Bridge Plugin 工具
#[derive(Parser)]
#[command(name = "frb_plugin_tool")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 创建新的 Flutter Rust Bridge 插件项目
    Create {
        /// 插件名称 (例如: my_plugin, hello_world)
        #[arg(short, long)]
        name: String,

        #[arg(short, long)]
        fvm_flutter_version: String,
    },
    /// 替换目录中的 REPLACE_PLUGIN_NAME 占位符
    Replace {
        /// 目标目录路径
        #[arg(short, long)]
        dir: PathBuf,

        /// 用于替换的插件名称
        #[arg(short, long)]
        name: String,
    },
    /// 配置 OHOS 开发环境（创建编译脚本和配置 Cargo）
    Presetup {
        /// 脚本存放目录 (例如: /Users/username/.ohos/script)
        #[arg(short, long)]
        script_path: String,

        /// OpenHarmony SDK 路径 (例如: /path/to/openharmony)
        #[arg(short, long)]
        openharmony_path: String,

        /// 强制替换现有配置
        #[arg(short, long)]
        force: bool,
    },
    AddSupport,
    AddCargoKit,
}

fn set_log_event() {
    // 初始化 tracing 子系统
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

struct PreSetup {
    //脚本目录
    script_path: String,
    //原生目录,例: /Users/ldd/hmos/command-line-tools/sdk/default/openharmony
    openharmony_path: String,

    //强制重新配置
    force: Option<bool>,
}
impl PreSetup {
    //准备工作
    async fn setup(self: Self) -> io::Result<()> {
        info!("开始配置 OHOS 开发环境...");

        // 1. 创建脚本目录（如果不存在）
        let script_dir = PathBuf::from(&self.script_path);
        if !script_dir.exists() {
            info!("创建脚本目录: {:?}", script_dir);
            create_dir_all(&script_dir).await?;
        }

        // 2. 创建 4 个脚本文件
        self.create_aarch64_clang_script().await?;
        self.create_aarch64_clang_plus_script().await?;
        self.create_x86_64_clang_script().await?;
        self.create_x86_64_clang_plus_script().await?;

        // 3. 配置 cargo config.toml
        self.setup_cargo_config().await?;

        info!("✅ OHOS 开发环境配置完成");
        Ok(())
    }

    // 创建 aarch64-unknown-linux-ohos-clang.sh
    async fn create_aarch64_clang_script(&self) -> io::Result<()> {
        let script_path = format!("{}/aarch64-unknown-linux-ohos-clang.sh", self.script_path);
        let content = format!(
            r#"#!/bin/sh
exec {}/native/llvm/bin/clang \
  -target aarch64-linux-ohos \
  --sysroot={}/native/sysroot \
  -D__MUSL__ \
  "$@"
"#,
            self.openharmony_path, self.openharmony_path
        );

        write(&script_path, content).await?;

        // 设置可执行权限
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = tokio::fs::metadata(&script_path).await?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            tokio::fs::set_permissions(&script_path, permissions).await?;
        }

        info!("✅ 创建脚本: {}", script_path);
        Ok(())
    }

    // 创建 aarch64-unknown-linux-ohos-clang++.sh
    async fn create_aarch64_clang_plus_script(&self) -> io::Result<()> {
        let script_path = format!("{}/aarch64-unknown-linux-ohos-clang++.sh", self.script_path);
        let content = format!(
            r#"#!/bin/sh
exec {}/native/llvm/bin/clang++ \
  -target aarch64-linux-ohos \
  --sysroot={}/native/sysroot \
  -D__MUSL__ \
  "$@"
"#,
            self.openharmony_path, self.openharmony_path
        );

        write(&script_path, content).await?;

        // 设置可执行权限
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = tokio::fs::metadata(&script_path).await?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            tokio::fs::set_permissions(&script_path, permissions).await?;
        }

        info!("✅ 创建脚本: {}", script_path);
        Ok(())
    }

    // 创建 x86_64-unknown-linux-ohos-clang.sh
    async fn create_x86_64_clang_script(&self) -> io::Result<()> {
        let script_path = format!("{}/x86_64-unknown-linux-ohos-clang.sh", self.script_path);
        let content = format!(
            r#"#!/bin/sh
exec {}/native/llvm/bin/clang \
  -target x86_64-linux-ohos \
  --sysroot={}/native/sysroot \
  -D__MUSL__ \
  "$@"
"#,
            self.openharmony_path, self.openharmony_path
        );

        write(&script_path, content).await?;

        // 设置可执行权限
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = tokio::fs::metadata(&script_path).await?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            tokio::fs::set_permissions(&script_path, permissions).await?;
        }

        info!("✅ 创建脚本: {}", script_path);
        Ok(())
    }

    // 创建 x86_64-unknown-linux-ohos-clang++.sh
    async fn create_x86_64_clang_plus_script(&self) -> io::Result<()> {
        let script_path = format!("{}/x86_64-unknown-linux-ohos-clang++.sh", self.script_path);
        let content = format!(
            r#"#!/bin/sh
exec {}/native/llvm/bin/clang++ \
  -target x86_64-linux-ohos \
  --sysroot={}/native/sysroot \
  -D__MUSL__ \
  "$@"
"#,
            self.openharmony_path, self.openharmony_path
        );

        write(&script_path, content).await?;

        // 设置可执行权限
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = tokio::fs::metadata(&script_path).await?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            tokio::fs::set_permissions(&script_path, permissions).await?;
        }

        info!("✅ 创建脚本: {}", script_path);
        Ok(())
    }

    // 配置 cargo config.toml
    async fn setup_cargo_config(&self) -> io::Result<()> {
        let home_dir = env::var("HOME").expect("无法获取 HOME 环境变量");
        let cargo_config_path = format!("{}/.cargo/config.toml", home_dir);

        let config_content = format!(
            r#"[target.aarch64-unknown-linux-ohos]
ar = "{}/native/llvm/bin/llvm-ar"
linker = "{}/aarch64-unknown-linux-ohos-clang.sh"

[target.x86_64-unknown-linux-ohos]
ar = "{}/native/llvm/bin/llvm-ar"
linker = "{}/x86_64-unknown-linux-ohos-clang.sh"

[env]
# 告诉 cc-rs 在为 aarch64-ohos 编译 C 代码时使用你的脚本
CC_aarch64_unknown_linux_ohos = "{}/aarch64-unknown-linux-ohos-clang.sh"
# 最好也为 C++ 设置一个，以防有 crate 需要编译 C++
CXX_aarch64_unknown_linux_ohos = "{}/aarch64-unknown-linux-ohos-clang++.sh"

# 同样为 x86_64 平台进行设置
CC_x86_64_unknown_linux_ohos = "{}/x86_64-unknown-linux-ohos-clang.sh"
CXX_x86_64_unknown_linux_ohos = "{}/x86_64-unknown-linux-ohos-clang++.sh"
"#,
            self.openharmony_path,
            self.script_path,
            self.openharmony_path,
            self.script_path,
            self.script_path,
            self.script_path,
            self.script_path,
            self.script_path
        );

        // 检查是否强制替换
        let force = self.force.unwrap_or(false);

        // 检查文件是否存在
        let cargo_config_pathbuf = PathBuf::from(&cargo_config_path);
        if cargo_config_pathbuf.exists() {
            // 读取现有内容
            let existing_content = read_to_string(&cargo_config_path).await?;

            // 检查是否已经配置过
            if existing_content.contains("target.aarch64-unknown-linux-ohos") {
                if force {
                    info!("🔄 强制模式：删除旧配置并重新写入");
                    // 移除旧的 OHOS 配置
                    let new_content = self.remove_ohos_config(&existing_content);
                    // 追加新配置
                    write(
                        &cargo_config_path,
                        format!("{}\n{}", new_content, config_content),
                    )
                    .await?;
                } else {
                    info!("⚠️  检测到已有 OHOS 配置，跳过写入（使用 --force 强制替换）");
                    return Ok(());
                }
            } else {
                info!("⚠️  cargo config.toml 已存在，将追加配置");
                // 追加新配置
                let mut file = OpenOptions::new()
                    .append(true)
                    .open(&cargo_config_path)
                    .await?;
                file.write_all(format!("\n{}", config_content).as_bytes())
                    .await?;
            }
        } else {
            // 创建 .cargo 目录（如果不存在）
            let cargo_dir = format!("{}/.cargo", home_dir);
            let cargo_dir_path = PathBuf::from(&cargo_dir);
            if !cargo_dir_path.exists() {
                create_dir_all(&cargo_dir_path).await?;
            }

            // 写入新文件
            write(&cargo_config_path, config_content).await?;
        }

        info!("✅ 配置 cargo config.toml: {}", cargo_config_path);
        Ok(())
    }

    // 移除现有的 OHOS 配置
    fn remove_ohos_config(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut skip = false;

        for line in lines {
            // 检测 OHOS 相关配置的开始
            if line.contains("[target.aarch64-unknown-linux-ohos]")
                || line.contains("[target.x86_64-unknown-linux-ohos]")
                || (line.contains("CC_aarch64_unknown_linux_ohos") && line.contains("="))
                || (line.contains("CXX_aarch64_unknown_linux_ohos") && line.contains("="))
                || (line.contains("CC_x86_64_unknown_linux_ohos") && line.contains("="))
                || (line.contains("CXX_x86_64_unknown_linux_ohos") && line.contains("="))
            {
                skip = true;
            }

            // 如果遇到新的配置段或空行后的新配置，停止跳过
            if skip && line.starts_with('[') && !line.contains("ohos") {
                skip = false;
            }

            // 如果当前行是 ar 或 linker 配置且包含 ohos，跳过
            if line.contains("ohos") && (line.contains("ar =") || line.contains("linker =")) {
                continue;
            }

            // 如果不需要跳过，添加到结果中
            if !skip {
                result.push(line);
            } else if line.trim().is_empty() {
                // 保留空行，但继续跳过
                continue;
            } else if line.contains("ohos") {
                // 跳过包含 ohos 的行
                continue;
            } else {
                // 遇到非 ohos 配置，停止跳过
                skip = false;
                result.push(line);
            }
        }

        result.join("\n").trim().to_string()
    }
}

struct OhosGenerate {
    plugin_name: String,
}
impl OhosGenerate {
    fn create(plugin_name: String) -> OhosGenerate {
        OhosGenerate { plugin_name }
    }

    //下载 ohos目录,和替换名称
    async fn fetch_github_temp(self: &Self) {
        info!("开始下载 ohos软件包");
        let mut path = env::current_dir().expect("获取执行目录失败");
        path.push(self.plugin_name.clone());
        let _ = cmd!(
            "git",
            "clone",
            format!("{}", OHOS_DIRECTORY_GIT_URL),
            "ohos"
        )
        .dir(path)
        .run()
        .expect("下载ohos模板文件失败");
    }

    async fn fetch_github_temp_current(self: &Self) {
        info!("开始下载 ohos软件包");
        let path = env::current_dir().expect("获取执行目录失败");
        let _ = cmd!(
            "git",
            "clone",
            format!("{}", OHOS_DIRECTORY_GIT_URL),
            "ohos"
        )
        .dir(path)
        .run()
        .expect("下载ohos模板文件失败");
    }

    async fn releace_plugin_name(self: &Self) {
        let plugin_name = self.plugin_name.clone();
        let mut path = env::current_dir().expect("获取执行目录失败");
        path.push(plugin_name.clone());
        path.push("ohos");
        info!("开始替换包名{:?},{}", path, plugin_name);
        let _ = replace_plugin_name_in_files(path, &plugin_name).await;
    }

    async fn releace_plugin_name_current(self: &Self) {
        let plugin_name = self.plugin_name.clone();
        let mut path = env::current_dir().expect("获取执行目录失败");
        path.push("ohos");
        info!("开始替换包名{:?},{}", path, plugin_name);
        let _ = replace_plugin_name_in_files(path, &plugin_name).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use duct::cmd;
    use tracing::warn;

    #[tokio::test]
    async fn test_setup() {
        set_log_event();
        let presetup = PreSetup {
            script_path: "/Users/ldd/.ohos/testscript".to_owned(),
            openharmony_path: "/Users/ldd/hmos/command-line-tools/sdk/default/openharmony"
                .to_owned(),
            force: Some(false),
        };
        let _ = presetup.setup().await;
    }

    #[tokio::test]
    async fn test_setup_force() {
        set_log_event();
        let presetup = PreSetup {
            script_path: "/Users/ldd/.ohos/testscript".to_owned(),
            openharmony_path: "/Users/ldd/hmos/command-line-tools/sdk/default/openharmony"
                .to_owned(),
            force: Some(true),
        };
        let _ = presetup.setup().await;
    }

    //测试下载和替换
    #[tokio::test]
    async fn test_ohos_director_download() {
        set_log_event();
        let plugin_name = String::from("plugin_test");
        let ohos_ = OhosGenerate::create(plugin_name.clone());
        ohos_.fetch_github_temp().await;

        //删除
        let mut dir = env::current_dir().unwrap();
        dir.push("ohos");
        let _ = replace_plugin_name_in_files(dir.clone(), &plugin_name).await;
        info!("开始删除 ohos目录");
        let r = tokio::fs::remove_dir_all(dir).await;
        if r.is_ok() {
            info!("删除成功");
        }
    }

    #[test]
    fn test_flutter_command_exists() {
        set_log_event();
        // This test will only run on Windows.
        // On other platforms, it will be skipped or marked as passed.
        if cfg!(windows) {
            println!("Current PATH: {:?}", env::var("PATH"));

            let result = cmd!("flutter.bat", "--version")
                .env("PATH", env::var("PATH").unwrap())
                .stdout_capture()
                .run();
            match result {
                Ok(_) => info!("成功"),
                Err(e) => warn!("失败:${e}"),
            }
        } else {
            info!("Skipping Flutter command test on non-Windows platform.");
        }
    }
}

fn get_path_env() -> String {
    env::var("PATH").unwrap()
}

/// 遍历目录并替换文件中的文本
///
/// # 参数
/// * `dir_path` - 要遍历的目录路径
/// * `file_name` - 用于替换 REPLACE_PLUGIN_NAME 的文本
///
/// # 示例
/// ```
/// let path = PathBuf::from("./my_project");
/// replace_plugin_name_in_files(path, "my_plugin").await;
/// ```
async fn replace_plugin_name_in_files(dir_path: PathBuf, file_name: &str) -> io::Result<()> {
    info!(
        "开始替换目录 {:?} 中的 REPLACE_PLUGIN_NAME 为 {}",
        dir_path, file_name
    );

    let mut replaced_count = 0;
    let mut file_count = 0;

    // 遍历目录中的所有文件
    for entry in WalkDir::new(&dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();

        // 读取文件内容
        match read_to_string(file_path).await {
            Ok(content) => {
                // 检查是否包含需要替换的文本
                if content.contains("REPLACE_PLUGIN_NAME") {
                    // 替换所有出现的 REPLACE_PLUGIN_NAME
                    let new_content = content.replace("REPLACE_PLUGIN_NAME", file_name);

                    // 写回文件
                    match write(file_path, new_content).await {
                        Ok(_) => {
                            replaced_count += 1;
                            info!("✅ 已替换文件: {:?}", file_path);
                        }
                        Err(e) => {
                            info!("⚠️  写入文件失败 {:?}: {}", file_path, e);
                        }
                    }
                }
                file_count += 1;
            }
            Err(_) => {}
        }
    }

    info!(
        "✅ 替换完成! 共扫描 {} 个文件，替换了 {} 个文件",
        file_count, replaced_count
    );
    Ok(())
}

//创建插件项目目录
async fn run_flutter_plugin_create(plugin_name: &str, fvm_flutter_version: &str) -> bool {
    info!("开始创建插件目录:{},请稍等...", plugin_name);
    if cfg!(windows) {
        let _ = cmd!("fvm", "use", fvm_flutter_version)
            .run()
            .expect("支持 fvm命令失败");
        let result = cmd!(
            "fvm",
            "flutter",
            "create",
            "--template=plugin_ffi",
            format!("{plugin_name}"),
            "--platforms",
            "android,ios,macos,windows,linux"
        )
        .dir(env::current_dir().expect("获取目录失败"))
        .env("PATH", get_path_env())
        .stdout_null()
        .run();
        result.is_ok()
    } else {
        let _ = cmd!("fvm", "use", fvm_flutter_version)
            .run()
            .expect("支持 fvm命令失败");
        let result = cmd!(
            "fvm",
            "flutter",
            "create",
            "--template=plugin_ffi",
            format!("{plugin_name}"),
            "--platforms",
            "android,ios,macos,windows,linux"
        )
        .dir(env::current_dir().expect("获取目录失败"))
        .env("PATH", get_path_env())
        .stdout_null()
        .run();
        result.is_ok()
    }
}

///给示例 example 项目添加 ohos支持
async fn add_example_project_init_ohos(plugin_name: String) {
    info!("开始给示例项目添加ohos模块支持");
    let mut p = env::current_dir().unwrap();
    p.push(format!("{plugin_name}"));
    p.push("example");
    cmd!("fvm", "flutter", "create", "--platforms=ohos", ".")
        .dir(env::current_dir().expect("获取目录失败"))
        .env("PATH", get_path_env())
        .stdout_null()
        .run()
        .unwrap();
    info!("✅开始给示例项目添加ohos模块支持");
}

///初始化git项目,并克隆cargokit项目
async fn init_git_config(plugin_name: &str) {
    info!("开始初始化(git):{plugin_name}");
    let mut p = env::current_dir().unwrap();
    p.push(plugin_name);
    cmd!("git", "init")
        .dir(p.clone())
        .stdout_null()
        .run()
        .unwrap();
    cmd!("git", "add", "--all")
        .dir(p.clone())
        .stdout_null()
        .run()
        .unwrap();
    cmd!("git", "commit", "-m", "initial commit")
        .dir(p.clone())
        .stdout_null()
        .stderr_null()
        .run()
        .unwrap();
    info!("开始下载cargokit...");
    cmd!(
        "git",
        "subtree",
        "add",
        "--prefix",
        "cargokit",
        "https://github.com/mdddj/cargokit_ohos",
        "master",
        "--squash"
    )
    .dir(p.clone())
    .stdout_null()
    .run()
    .unwrap();
    info!("✅初始化git环境成功");
}

async fn add_rust_lib_project(plugin_name: &str) {
    info!("开始初始化rust lib项目");
    let mut p = env::current_dir().unwrap();
    p.push(plugin_name);
    cmd!(
        "cargo",
        "new",
        "rust",
        "--lib",
        "--name",
        format!("{plugin_name}")
    )
    .dir(p.clone())
    .stdout_null()
    .run()
    .unwrap();
    p.push("rust");
    p.push("Cargo.toml");
    let txt = get_temp("Cargo.toml", |ctx| ctx.insert("name", plugin_name)).await;
    let mut cargo_file = File::create(p).await.expect("获取cargo.toml文件失败");
    cargo_file
        .write_all(txt.as_bytes())
        .await
        .expect("写入配置失败");
    info!("✅创建rust包成功");
}

///从github上加载
async fn fetch_github_temp_file_string(file_name: &str) -> Result<String, reqwest::Error> {
    let url = format!(
        "https://raw.githubusercontent.com/mdddj/frb_plugin_tool/refs/heads/ohos/temp/{file_name}"
    );
    info!("开始从github下载模板:{file_name}");
    let response = reqwest::get(url.as_str()).await?.text().await?;
    info!("✅加载模板引擎文本成功 {file_name}");
    Ok(response)
}

///获取模板函数
async fn get_temp<F: FnMut(&mut Context) -> ()>(file_name: &str, mut handle: F) -> String {
    let mut tera = Tera::default();
    let txt = fetch_github_temp_file_string(file_name).await;
    match txt {
        Ok(temp_txt) => {
            tera.add_raw_template(file_name, &temp_txt).unwrap();
            let mut ctx = Context::new();
            handle(&mut ctx);
            let txt = tera.render(file_name, &ctx).unwrap();
            txt
        }
        Err(err) => panic!("加载失败{}", err),
    }
}

///添加frb配置文件
async fn add_frb_yaml_file(plugin_name: &str) {
    let mut dir = env::current_dir().unwrap();
    dir.push(plugin_name);
    let file_name = "flutter_rust_bridge.yaml";
    dir.push(file_name);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dir)
        .await
        .unwrap();
    let text = get_temp(file_name, |_| {}).await;
    file.write_all(text.as_bytes())
        .await
        .expect("写入frb配置失败");
    info!("✅写入flutter_rust_bridge.yaml成功");
}

///添加macos脚本
async fn add_macos_script(plugin_name: &str) {
    let file_name = format!("{plugin_name}.podspec");
    let mut dir = env::current_dir().unwrap();
    dir.push(plugin_name);
    dir.push("macos");
    dir.push(&file_name);
    let mut file = File::create(dir)
        .await
        .expect(&format!("读取{file_name}失败"));
    let temp = get_temp("plugin.podspec", |ctx| ctx.insert("name", plugin_name)).await;
    file.write_all(temp.as_bytes())
        .await
        .expect(&format!("写入{file_name}配置失败"));
    info!("✅添加macos脚本成功");
}

///添加ios脚本
async fn add_ios_script(plugin_name: &str) {
    let file_name = format!("{plugin_name}.podspec");
    let mut dir = env::current_dir().unwrap();
    dir.push(plugin_name);
    dir.push("ios");
    dir.push(&file_name);
    let mut file = File::create(dir)
        .await
        .expect(&format!("读取{file_name}失败"));
    let temp = get_temp("plugin.podspec", |ctx| ctx.insert("name", plugin_name)).await;
    file.write_all(temp.as_bytes())
        .await
        .expect(&format!("写入{file_name}配置失败"));
    info!("✅添加ios脚本成功");
}

///添加windows脚本
async fn add_windows_script(plugin_name: &str) {
    let mut dir = env::current_dir().unwrap();
    dir.push(plugin_name);
    dir.push("windows");
    dir.push("CMakeLists.txt");
    let mut file = File::create(dir)
        .await
        .expect(&"读取windows CMakeLists.txt失败".to_string());
    let temp = get_temp("cmake.txt", |ctx| ctx.insert("name", plugin_name)).await;
    file.write_all(temp.as_bytes())
        .await
        .expect(&"写入windows CMakeLists.txt配置失败".to_string());
    info!("✅添加windows脚本成功")
}

///添加linux脚本
async fn add_linux_script(plugin_name: &str) {
    let mut dir = env::current_dir().unwrap();
    dir.push(plugin_name);
    dir.push("linux");
    dir.push("CMakeLists.txt");
    let mut file = File::create(dir)
        .await
        .expect(&"读取linux CMakeLists.txt失败".to_string());
    let temp = get_temp("cmake.txt", |ctx| ctx.insert("name", plugin_name)).await;
    file.write_all(temp.as_bytes())
        .await
        .expect(&"写入linux CMakeLists.txt配置失败".to_string());
    info!("✅添加linux脚本成功")
}

///添加android脚本
async fn add_android_script(plugin_name: &str) {
    let mut dir = env::current_dir().unwrap();
    dir.push(plugin_name);
    dir.push("android");
    dir.push("build.gradle");
    let mut file = File::create(dir)
        .await
        .expect(&"读取android build.gradle失败".to_string());
    let temp = get_temp("build.gradle", |ctx| ctx.insert("name", plugin_name)).await;
    file.write_all(temp.as_bytes())
        .await
        .expect(&"写入build.gradle配置失败".to_string());
    info!("✅添加android脚本成功")
}

///添加yaml依赖
async fn add_pubspec_script(plugin_name: &str) {
    let mut dir = env::current_dir().unwrap();
    dir.push(plugin_name);
    dir.push("pubspec.yaml");
    let mut file = File::create(dir)
        .await
        .expect(&"读取pubspec.yaml失败".to_string());
    let temp = get_temp("pubspec.yaml", |ctx| ctx.insert("name", plugin_name)).await;
    file.write_all(temp.as_bytes())
        .await
        .expect(&"写入pubspec.yaml配置失败".to_string());
    info!("✅添加yaml依赖成功");
}

///配置Rust的OHOS target支持
async fn setup_rust_ohos_targets() {
    info!("开始配置Rust OHOS target支持...");

    // 添加 aarch64-unknown-linux-ohos target
    let result_aarch64 = cmd!("rustup", "target", "add", "aarch64-unknown-linux-ohos")
        .stdout_null()
        .run();

    if result_aarch64.is_ok() {
        info!("✅添加 aarch64-unknown-linux-ohos target 成功");
    } else {
        info!("⚠️ aarch64-unknown-linux-ohos target 可能已存在或添加失败");
    }

    // 添加 x86_64-unknown-linux-ohos target (可选，用于模拟器)
    let result_x86_64 = cmd!("rustup", "target", "add", "x86_64-unknown-linux-ohos")
        .stdout_null()
        .run();

    if result_x86_64.is_ok() {
        info!("✅添加 x86_64-unknown-linux-ohos target 成功");
    } else {
        info!("⚠️ x86_64-unknown-linux-ohos target 可能已存在或添加失败");
    }
}

///添加示例rust目录和文件 /api/hello.rs
async fn add_test_rs_file(plugin_name: &str) {
    let mut dir = env::current_dir().unwrap();
    dir.push(plugin_name);
    dir.push("rust");
    dir.push("src");
    dir.push("api");
    create_dir(&dir)
        .await
        .expect("创建rust/api目录失败.请手动创建");
    dir.push("mod.rs");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&dir)
        .await
        .expect("创建mod.rs失败");
    file.write_all("pub mod hello;".as_bytes())
        .await
        .expect("写入mod.rs失败");

    dir.pop();
    dir.push("hello.rs");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&dir)
        .await
        .expect("创建hello.rs失败");
    file.write_all(
        r#"
    pub fn hello(hello: &str) {
        println!("hello world!");
    }
    "#
        .as_bytes(),
    )
    .await
    .expect("写入hello.rs失败");

    //声明pod mod api;

    dir.pop();
    dir.pop();
    dir.push("lib.rs");
    let mut file = File::create(&dir).await.expect("打开lib.rs失败");
    file.write_all("pub mod api;".as_bytes())
        .await
        .expect("写入lib.rs失败");
    info!("✅写入rust test api成功")
}

/// 执行创建插件的完整流程
async fn execute_create_plugin(plugin_name: String, fvm_flutter_version: String) {
    let plugin_name = Arc::new(plugin_name);
    let is_ok = run_flutter_plugin_create(&plugin_name, &fvm_flutter_version).await;

    if !is_ok {
        info!("❌ 创建插件失败");
        return;
    }

    //给示例项目添加 ohos支持
    add_example_project_init_ohos(plugin_name.as_ref().to_owned()).await;

    let name = Arc::clone(&plugin_name);
    let git_task = task::spawn(async move { init_git_config(&name).await });
    let _ = git_task.await;

    let yaml_name = Arc::clone(&plugin_name);
    let add_rust_name = Arc::clone(&plugin_name);
    let macos_name = Arc::clone(&plugin_name);
    let ios_name = Arc::clone(&plugin_name);
    let windows_name = Arc::clone(&plugin_name);
    let linux_name = Arc::clone(&plugin_name);
    let android_name = Arc::clone(&plugin_name);
    let pubspc_name = Arc::clone(&plugin_name);
    let ohos_name = Arc::clone(&plugin_name);
    let test_name = Arc::clone(&plugin_name);

    // 首先配置 Rust OHOS targets
    setup_rust_ohos_targets().await;

    let ohos_code_fetch = OhosGenerate::create(ohos_name.as_ref().clone());
    ohos_code_fetch.fetch_github_temp().await;
    ohos_code_fetch.releace_plugin_name().await;

    let tasks = vec![
        task::spawn(async move { add_rust_lib_project(&add_rust_name).await }),
        task::spawn(async move { add_frb_yaml_file(&yaml_name).await }),
        task::spawn(async move { add_macos_script(&macos_name).await }),
        task::spawn(async move { add_ios_script(&ios_name).await }),
        task::spawn(async move { add_windows_script(&windows_name).await }),
        task::spawn(async move { add_linux_script(&linux_name).await }),
        task::spawn(async move { add_android_script(&android_name).await }),
        task::spawn(async move { add_pubspec_script(&pubspc_name).await }),
    ];
    future::join_all(tasks).await;

    info!("✅项目创建成功,开始写入test文件");
    let add_file_task = vec![task::spawn(
        async move { add_test_rs_file(&test_name).await },
    )];
    future::join_all(add_file_task).await;

    info!("🎉 所有任务完成！插件 {} 已创建成功", plugin_name);
}

/// 定义可能发生的错误类型，使错误处理更清晰。
#[derive(Error, Debug)]
pub enum PubspecError {
    /// IO 错误，例如文件不存在或没有读取权限。
    #[error("无法读取 pubspec.yaml 文件: {0}")]
    Io(#[from] std::io::Error),

    /// YAML 解析错误，例如文件格式不正确或缺少 `name` 字段。
    #[error("解析 pubspec.yaml 失败: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// 定义我们关心的 `pubspec.yaml` 文件结构。
/// 我们只需要 `name` 字段，所以只定义它。
/// `#[derive(Deserialize)]` 宏会自动为我们实现反序列化逻辑。
#[derive(Deserialize, Debug)]
struct Pubspec {
    name: String,
}

/// 解析当前目录中的 pubspec.yaml 文件，并返回顶级的 `name` 属性值。
///
/// # 返回
/// - `Ok(String)`: 如果成功，返回移除前后空格的 `name` 值。
/// - `Err(PubspecError)`: 如果发生错误，返回具体的错误类型。
pub fn get_pubspec_name() -> Result<String, PubspecError> {
    // 1. 获取当前执行目录的路径
    let mut path = std::env::current_dir()?;

    // 2. 将 "pubspec.yaml" 文件名附加到路径末尾
    path.push("pubspec.yaml");

    // 3. 读取文件内容到字符串中
    // `?` 操作符会在发生 IO 错误时自动返回 `Err(PubspecError::Io)`
    let content = fs::read_to_string(&path)?;

    // 4. 使用 serde_yaml 将文件内容解析到 `Pubspec` 结构体中
    // 如果 YAML 格式错误或缺少 `name` 字段，这里会返回错误。
    // `?` 操作符会在发生解析错误时自动返回 `Err(PubspecError::Yaml)`
    let pubspec: Pubspec = serde_yaml::from_str(&content)?;

    // 5. 获取 `name` 字段的值，使用 `trim()` 移除前后空格，
    //    并使用 `to_string()` 转换回一个拥有的 String 类型。
    let trimmed_name = pubspec.name.trim().to_string();

    // 6. 返回成功的结果
    Ok(trimmed_name)
}

#[tokio::main]
async fn main() {
    set_log_event();

    let cli = Cli::parse();
    let command = cli.command;

    match command {
        Commands::Create {
            name,
            fvm_flutter_version,
        } => {
            info!("开始创建插件: {}", name);
            execute_create_plugin(name, fvm_flutter_version).await;
        }
        Commands::Replace { dir, name } => {
            info!("开始替换目录 {:?} 中的占位符为 {}", dir, name);
            match replace_plugin_name_in_files(dir, &name).await {
                Ok(_) => info!("✅ 替换完成"),
                Err(e) => info!("❌ 替换失败: {}", e),
            }
        }
        Commands::Presetup {
            script_path,
            openharmony_path,
            force,
        } => {
            info!("开始配置 OHOS 开发环境");
            info!("脚本目录: {}", script_path);
            info!("OpenHarmony 路径: {}", openharmony_path);
            info!("强制模式: {}", force);

            let presetup = PreSetup {
                script_path,
                openharmony_path,
                force: Some(force),
            };

            match presetup.setup().await {
                Ok(_) => info!("🎉 OHOS 开发环境配置成功！"),
                Err(e) => info!("❌ 配置失败: {}", e),
            }
        }
        Commands::AddSupport => {
            info!("开始给当前插件添加鸿蒙支持");
            //获取插件名称(从执行目录中读取 pubspec.yaml, name属性读取)
            match get_pubspec_name() {
                Ok(plugin_name) => {
                    let ohos_gen = OhosGenerate::create(plugin_name);
                    ohos_gen.fetch_github_temp_current().await;
                    ohos_gen.releace_plugin_name_current().await;
                }
                Err(e) => match e {
                    PubspecError::Io(error) => error!("pubspec.yaml文件不存在{:?}", error),
                    PubspecError::Yaml(error) => error!("解析 pubspec.yaml失败,{:?}", error),
                },
            }
        }
        Commands::AddCargoKit => {
            //重新添加鸿蒙版本的cargokit
            let mut curr_dir = env::current_dir().expect("获取执行目录失败");
            let dir = curr_dir.clone();
            curr_dir.push("cargokit");
            if curr_dir.exists() {
                info!("删除旧版本 cargokit");
                tokio::fs::remove_dir_all(curr_dir).await.unwrap();
                info!("✅删除旧版本 cargokit成功")
            }
            info!("开始添加鸿蒙版本cargokit");
            cmd!(
                "git",
                "subtree",
                "add",
                "--prefix",
                "cargokit",
                "https://github.com/mdddj/cargokit_ohos",
                "master",
                "--squash"
            )
            .dir(dir)
            .run()
            .expect("下载cargokit失败");
            info!("✅下载成功");
        }
    }
}
