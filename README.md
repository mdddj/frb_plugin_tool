# frb_plugin_tool_ohos

快速生成 Flutter Rust Bridge (FRB) 插件项目的命令行工具，支持多平台包括 HarmonyOS Next (鸿蒙系统)。



## 🎉 特性

✨ **全平台支持**
- Android
- iOS
- macOS
- Windows
- Linux
- **HarmonyOS Next (鸿蒙)**

✨ **自动化配置**
- 自动生成项目结构
- 配置 Rust OHOS targets
- 生成平台构建脚本
- 自动配置开发环境

> 注意: 因为要从 GitHub 拉取相关依赖，网络环境需要能访问 GitHub，否则会失败

## 📦 安装

```bash
cargo install frb_plugin_tool_ohos
```

## 🚀 使用

### 查看帮助

```bash
frb_plugin_tool_ohos --help
```

### 命令列表

frb_plugin_tool_ohos 提供三个主要命令：

## 1. `create` - 创建新插件项目

创建一个新的 Flutter Rust Bridge 插件项目，支持所有平台。

### 用法

```bash
frb_plugin_tool_ohos create --name <插件名称> --fvm-flutter-version <Flutter版本>
```

### 参数

- `--name` / `-n`: 插件名称（必需）
  - 例如: `my_plugin`, `hello_world`
  - 必须是合法的 Dart 包名（小写字母、数字、下划线）

- `--fvm-flutter-version` / `-f`: FVM Flutter 版本（必需）
  - 例如: `custom_3.27-oh`

### 示例

```bash
# 创建名为 my_plugin 的插件
frb_plugin_tool_ohos create -n my_plugin -f custom_3.27-oh
```

### 创建内容

该命令会自动：
- ✅ 创建 Flutter 插件项目结构
- ✅ 初始化 Git 仓库
- ✅ 添加 Cargokit 支持
- ✅ 创建 Rust lib 项目
- ✅ 配置 Flutter Rust Bridge
- ✅ 生成各平台构建脚本（iOS、Android、macOS、Windows、Linux）
- ✅ 下载并配置 OHOS 平台支持
- ✅ 配置 Rust OHOS targets
- ✅ 生成示例代码
- ✅ 生成 `OHOS_SETUP.md` 配置指南

---

## 2. `replace` - 替换占位符

遍历指定目录，将文件中的 `REPLACE_PLUGIN_NAME` 占位符替换为实际的插件名称。

### 用法

```bash
frb_plugin_tool_ohos replace --dir <目录路径> --name <插件名称>
```

### 参数

- `--dir` / `-d`: 目标目录路径（必需）
  - 例如: `./my_project/ohos`

- `--name` / `-n`: 用于替换的插件名称（必需）
  - 例如: `my_plugin`

### 示例

```bash
# 替换 ohos 目录中的占位符
frb_plugin_tool_ohos replace -d ./my_plugin/ohos -n my_plugin
```

### 功能说明

- 递归遍历目录中的所有文件
- 查找并替换所有 `REPLACE_PLUGIN_NAME` 出现
- 自动跳过二进制文件
- 显示替换进度和统计信息

---

## 3. `presetup` - 配置 OHOS 开发环境

自动配置 HarmonyOS Next 开发环境，包括创建编译脚本和配置 Cargo。

### 用法

```bash
frb_plugin_tool_ohos presetup \
  --script-path <脚本目录> \
  --openharmony-path <SDK路径> \
  [--force]
```

### 参数

- `--script-path` / `-s`: 脚本存放目录（必需）
  - 例如: `/Users/username/.ohos/script`
  - 如果目录不存在会自动创建

- `--openharmony-path` / `-o`: OpenHarmony SDK 路径（必需）
  - 例如: `/Users/username/hmos/command-line-tools/sdk/default/openharmony`
  - 这是 SDK 的 native 目录的父路径

- `--force` / `-f`: 强制替换现有配置（可选）
  - 默认: `false`
  - 如果检测到已有配置，使用此选项强制覆盖

### 示例

**普通模式（检测已有配置）：**
```bash
frb_plugin_tool_ohos presetup \
  -s ~/.ohos/script \
  -o ~/hmos/command-line-tools/sdk/default/openharmony
```

**强制模式（覆盖现有配置）：**
```bash
frb_plugin_tool_ohos presetup \
  -s ~/.ohos/script \
  -o ~/hmos/command-line-tools/sdk/default/openharmony \
  --force
```

### 配置内容

该命令会自动：

1. **创建编译脚本**（在 `script_path` 目录）：
   - `aarch64-unknown-linux-ohos-clang.sh`
   - `aarch64-unknown-linux-ohos-clang++.sh`
   - `x86_64-unknown-linux-ohos-clang.sh`
   - `x86_64-unknown-linux-ohos-clang++.sh`
   - 自动设置可执行权限（755）

2. **配置 Cargo**（`~/.cargo/config.toml`）：
   - 添加 `aarch64-unknown-linux-ohos` target 配置
   - 添加 `x86_64-unknown-linux-ohos` target 配置
   - 配置 linker 和 ar 工具路径
   - 设置 CC/CXX 环境变量

3. **智能处理**：
   - 检测已有配置（普通模式）
   - 强制替换旧配置（强制模式）
   - 自动创建必要的目录

---

## 4. 给当前插件项目添加鸿蒙支持


```bash
frb_plugin_tool_ohos add-support
```

> 需要在插件项目的根目录下执行 (要读取 pubspec.yaml下面的 name属性)


## 📝 完整工作流程示例

### 1. 配置 OHOS 开发环境

```bash
# 首次配置
frb_plugin_tool_ohos presetup \
  -s ~/.ohos/script \
  -o ~/hmos/command-line-tools/sdk/default/openharmony

# 如果需要更新配置
frb_plugin_tool_ohos presetup \
  -s ~/.ohos/script \
  -o ~/hmos/command-line-tools/sdk/default/openharmony \
  --force
```

### 2. 创建新插件项目

```bash
frb_plugin_tool_ohos create -n my_awesome_plugin -f custom_3.27-oh
```

### 3. 开发和构建

```bash
cd my_awesome_plugin

# 查看生成的鸿蒙配置指南
cat OHOS_SETUP.md

# 构建项目
flutter build ohos
```

---

## 🔧 环境要求

- Rust 工具链
- Flutter SDK
- Git
- FVM (可选，用于 Flutter 版本管理)
- HarmonyOS SDK (用于鸿蒙平台开发)


## 💬 反馈与支持

QQ 群: 706438100
