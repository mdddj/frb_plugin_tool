# frb_plugin_tool


## 可以让你写的 flutter rust 包上传到pub.dev的生成工具


qq群反馈：706438100

快速生成dart rust ffi 项目结构脚本

## 🎉 新增功能

✨ **现已支持 HarmonyOS Next (鸿蒙系统)！**

本工具现在可以自动生成支持鸿蒙平台的 Flutter Rust Bridge 插件项目，包括：
- 自动添加 `ohos` 平台支持
- 配置 Rust OHOS targets (aarch64-unknown-linux-ohos, x86_64-unknown-linux-ohos)
- 生成鸿蒙平台构建脚本
- 自动生成鸿蒙配置指南文档

> 注意: 因为要从 github 拉取相关依赖，网络环境需要能访问 github, 否则会失败

# 安装

```bash
cargo install frb_plugin_tool
```


# 运行

```bash
frb_plugin_tool
```

运行后会自动创建支持以下平台的 Flutter 插件项目：
- Android
- iOS
- macOS
- Windows
- Linux
- **HarmonyOS Next (鸿蒙)**

## 鸿蒙平台配置

项目创建完成后，会在项目根目录生成 `OHOS_SETUP.md` 文件，其中包含详细的鸿蒙平台配置步骤。

主要配置步骤：
1. 下载并安装 OHOS SDK
2. 配置 Rust 交叉编译脚本
3. 设置 Cargo 配置文件
4. 设置环境变量

详细配置说明请参考生成的 `OHOS_SETUP.md` 文件或查看 [frb-ohos.md](./frb-ohos.md)


