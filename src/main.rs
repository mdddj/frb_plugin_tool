use duct::cmd;
use futures::future;
use std::{env, io, path::PathBuf, sync::Arc};
use tera::{Context, Tera};
use tokio::{
    fs::{create_dir, read_to_string, write, File, OpenOptions},
    io::AsyncWriteExt,
    task,
};
use tracing::info;
use tracing_subscriber::fmt;
use walkdir::WalkDir;

//插件模板文件仓库地址
const OHOS_DIRECTORY_GIT_URL: &str = "https://github.com/mdddj/flutter_rust_plugin_ohos_temp";

fn set_log_event() {
    // 初始化 tracing 子系统
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
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
        let _ = replace_plugin_name_in_files(path, &plugin_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use duct::cmd;
    use tracing::warn;

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
//获取插件名
fn get_plugin_name() -> String {
    let mut input = String::new();
    info!("请输入合法dart插件名字(例:hello_dart,hi_ldd_plugin):");
    let _ = io::stdin().read_line(&mut input).expect("读取项目名失败");
    input = input.trim().to_string();
    input
}

//创建插件项目目录
async fn run_flutter_plugin_create(plugin_name: &str) -> bool {
    info!("开始创建插件目录:{},请稍等...", plugin_name);
    if cfg!(windows) {
        let result = cmd!(
            "flutter.bat",
            "create",
            "--template=plugin_ffi",
            format!("{plugin_name}"),
            "--platforms",
            "android,ios,macos,windows,linux,ohos"
        )
        .dir(env::current_dir().expect("获取目录失败"))
        .env("PATH", get_path_env())
        .stdout_null()
        .run();
        result.is_ok()
    } else {
        let _ = cmd!("fvm", "use", "custom_3.27-oh").run();
        let result = cmd!(
            "fvm",
            "flutter",
            "create",
            "--template=plugin_ffi",
            format!("{plugin_name}"),
            "--platforms",
            "android,ios,macos,windows,linux,ohos"
        )
        .dir(env::current_dir().expect("获取目录失败"))
        .env("PATH", get_path_env())
        .stdout_null()
        .run();
        result.is_ok()
    }
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
    let url =
        format!("https://raw.githubusercontent.com/mdddj/frb_plugin_tool/main/temp/{file_name}");
    info!("开始从github下载模板:{url}");
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

///生成OHOS配置说明文档
async fn generate_ohos_setup_guide(plugin_name: &str) {
    let mut dir = env::current_dir().unwrap();
    dir.push(plugin_name);
    dir.push("OHOS_SETUP.md");

    let guide_content = r#"# HarmonyOS Next 配置指南

本插件已支持 HarmonyOS Next 平台。请按照以下步骤完成配置：

## 1. 安装 OHOS SDK

下载并安装 OHOS SDK：
https://developer.huawei.com/consumer/cn/download/

## 2. 配置 Rust 交叉编译

### 创建编译脚本

在 `~/.ohos/script/` 目录下创建以下脚本文件：

#### aarch64-unknown-linux-ohos-clang.sh
```bash
#!/bin/sh
exec /usr/local/ohos-sdk/linux/native/llvm/bin/clang \
  -target aarch64-linux-ohos \
  --sysroot=/usr/local/ohos-sdk/linux/native/sysroot \
  -D__MUSL__ \
  "$@"
```

#### aarch64-unknown-linux-ohos-clang++.sh
```bash
#!/bin/sh
exec /usr/local/ohos-sdk/linux/native/llvm/bin/clang++ \
  -target aarch64-linux-ohos \
  --sysroot=/usr/local/ohos-sdk/linux/native/sysroot \
  -D__MUSL__ \
  "$@"
```

### 添加可执行权限
```bash
chmod +x ~/.ohos/script/*.sh
```

### 配置 Cargo

在 `~/.cargo/config.toml` 中添加：

```toml
[target.aarch64-unknown-linux-ohos]
ar = "/usr/local/ohos-sdk/linux/native/llvm/bin/llvm-ar"
linker = ".ohos/script/aarch64-unknown-linux-ohos-clang.sh"
```

**注意**：将 `/usr/local/ohos-sdk/linux` 替换为你的 OHOS SDK native 目录的父文件夹路径。

## 3. 设置环境变量

```bash
export AR=/usr/local/ohos-sdk/linux/native/llvm/bin/llvm-ar
export CC="~/.ohos/script/aarch64-unknown-linux-ohos-clang.sh"
```

## 4. 构建项目

配置完成后，使用 Flutter 命令构建 OHOS 平台：

```bash
flutter build ohos
```

## 参考资料

- [Flutter Rust Bridge 文档](https://cjycode.com/flutter_rust_bridge/)
- [HarmonyOS 开发者文档](https://developer.huawei.com/consumer/cn/doc/harmonyos-guides-V5/application-dev-guide-V5)
"#;

    let mut file = File::create(dir).await.expect("创建OHOS_SETUP.md失败");
    file.write_all(guide_content.as_bytes())
        .await
        .expect("写入OHOS_SETUP.md失败");
    info!("✅生成鸿蒙配置指南成功");
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

#[tokio::main]
async fn main() {
    set_log_event();
    let plugin_name = Arc::new(get_plugin_name());
    let is_ok = run_flutter_plugin_create(&plugin_name).await;
    if is_ok {
        let name = Arc::clone(&plugin_name);

        let git_task = task::spawn(async move { init_git_config(&name).await });
        let _ = git_task.await;

        // add_frb_yaml_file(&plugin_name).await;
        // add_macos_script(&plugin_name).await;
        // add_ios_script(&plugin_name).await;
        // add_windows_script(&plugin_name).await;
        // add_linux_script(&plugin_name).await;
        // add_android_script(&plugin_name).await;
        // add_pubspec_script(&plugin_name).await;
        // add_test_rs_file(&plugin_name).await;
        let yaml_name = Arc::clone(&plugin_name);
        let add_rust_name = Arc::clone(&plugin_name);
        let macos_name = Arc::clone(&plugin_name);
        let ios_name = Arc::clone(&plugin_name);
        let windows_name = Arc::clone(&plugin_name);
        let linux_name = Arc::clone(&plugin_name);
        let android_name = Arc::clone(&plugin_name);
        let pubspc_name = Arc::clone(&plugin_name);
        let ohos_name = Arc::clone(&plugin_name);
        let ohos_guide_name = Arc::clone(&plugin_name);
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
            task::spawn(async move { generate_ohos_setup_guide(&ohos_guide_name).await }),
        ];
        future::join_all(tasks).await;
        info!("✅项目创建成功,开始写入test文件");
        let add_file_task = vec![task::spawn(
            async move { add_test_rs_file(&test_name).await },
        )];
        future::join_all(add_file_task).await;
    }
}
