use duct::cmd;
use futures::future;
use std::{env, io, sync::Arc};
use tera::{Context, Tera};
use tokio::{
    fs::{create_dir, File, OpenOptions},
    io::AsyncWriteExt,
    task,
};
use tracing::info;
use tracing_subscriber::fmt;

fn set_log_event() {
    // 初始化 tracing 子系统
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn get_path_env() -> String {
    env::var("PATH").unwrap()
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
    info!("开始创建插件目录:{}", plugin_name);
    let result = cmd!(
        "flutter",
        "create",
        "--template=plugin_ffi",
        format!("{plugin_name}"),
        "--platforms",
        "android,ios,macos,windows,linux"
    )
    .dir(env::current_dir().expect("获取目录失败"))
    .env("env", get_path_env())
    .stdout_null()
    .run();
    result.is_ok()
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
        "https://github.com/irondash/cargokit.git",
        "main",
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
        let test_name = Arc::clone(&plugin_name);

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

    }
}
