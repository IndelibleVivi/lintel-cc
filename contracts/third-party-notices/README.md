# Rust 分发 notices 的固定来源

[prepare-rust-notices.mjs](../../scripts/prepare-rust-notices.mjs) 是生成入口：

```sh
node scripts/prepare-rust-notices.mjs
node --test tests/rust_notices.test.mjs
```

生成器离线读取三套锁定 Cargo graph、本机 Cargo source、当前 `rustc` 的 `rust-docs` 和本目录的固定证据，输出到 ignored `apps/desktop/src-tauri/third-party-notices/`。它保留 desktop/native-host 目标以及两个 Linux musl runner 的 normal/build 可达依赖，包括 build tools 与 procedural macros；这是保守集合，不是 exact linked SBOM。缺少许可正文、明确声明的 `license-file`、固定补充材料或标准库 notices 时，生成失败。生成文件不作为维护源，也不进 Git。

`rust-supplements.json` 将发布包缺少的许可正文绑定到精确 crate 名称、版本、声明和 `.cargo_vcs_info.json` upstream commit。`texts/` 保存原样上游正文、授权声明或注明范围的源文件许可注释；SHA-256 用于防止这些固定材料被误改。标准许可正文仅用于有明确上游授权声明的条目，不替换不同许可证，不按 package authors 猜测 copyright holder，也不填写标准模板的占位符。上游多重许可、NOTICE 和嵌套第三方条款分别保留。这里只维护第三方材料，不改变 Lintel 原创材料的授权。

更新依赖后重新生成；若出现缺项，应核对该精确版本的发布包与上游来源，更新对应证据和正文，不能复用旧版本补丁绕过失败。`native-runtime.json` 与对应正文固定当前 runner 构建采用的 Zig 0.16.0 分发包：包括 Zig、musl、unwind 许可及 runtime/supporting source 中的版权、许可注释；来源路径均相对于公开 wheel，提取时已核对安装包 RECORD。更换 Zig 或 native runtime 必须准备对应版本的材料并更新这个明确的 profile。

当前 Rust 标准库材料来自准备时的工具链；固定 Zig snapshot 也只证明保留了那套来源的 notices。两者都不会自动证明某个 binary 的构建 provenance。打包方仍须使实际 desktop、host、runner 的构建工具链与这些记录一致；复用旧 binary 时保留其原构建工具链的 notices。公开源码链接及原有许可继续适用于上游材料，生成成功不代表正式发布、签名、公证或部署已经完成。
