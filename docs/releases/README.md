# 发行流程

原 `0.1.0-rc.1` 草稿已暂停发布；设置修复与任务/DPI 改动不在旧包内。需要在发行前生成新的候选版并重新完成验收，不代表稳定版已发布。[候选验收记录](0.1.0-rc.1.md)逐项区分已验证、未验证和不随包分发的内容。

## 构建候选包

维护者在 Actions → Windows build and core checks → Run workflow 中选择候选分支，勾选 `release_candidate`。它调用独立的 `release.yml`，不会扩展日常测试，也不会提交录像或自动公开发行。

发行任务执行：格式和 Clippy（仅豁免原生控件参数数量与像素分块风格提示）、五项 Windows 核心测试、四组暂存的设置/任务/绘制/DPI 回归、单项 Game Bar 操作顺序回归、三组现有发行网络回归、MSVC release 构建、普通运行模式下真实 SendInput 快捷键/屏幕截图/回环决策/键鼠穿透、隐藏后台 fresh/after-use 各 60 次采样及常驻浮窗 60 次采样、现有原生 UI/透明度检查，然后打包。

CIM 查询使实际采样区间超过 60 秒，原始结果保留实际时长。脚本验证标准 WM_CLOSE 后的退出，不认证托盘菜单键盘导航。合成桌面上的焦点和输入通过也不等于游戏兼容性通过。该任务不调用真实模型、不使用真实密钥；不会通过注入或反作弊规避获取结果。

产物 `release-candidate-windows-x64` 包含：

- `blink-verdict-0.1.0-rc.1-windows-x64.zip`
- `SHA256SUMS.txt`

ZIP 内只有程序、快速使用说明、更新记录、MIT 许可、依赖许可、Rust 标准库声明及 `build-info.json`。清单记录版本、源码提交、工具链、锁文件摘要和各文件 SHA-256；校验和校验传输内容，不等于代码签名。实验性 Game Bar、演示影片、截图、配置、密钥、测试工具和模型权重不随包分发。

`release-validation-evidence` 保存合成桌面验证和资源采样 JSON。候选包未使用 Authenticode 签名，未提供 ARM64 / x86 产物。

## 准备发行页

下载并核对上述 ZIP 与校验文件，先创建 GitHub **draft prerelease**，上传同一构建产生的附件。发行说明包含新增功能、系统/架构、安装与更新方式、未签名状态、已知限制及验收记录。准备工作不自动发布草稿、不创建稳定标签、不覆盖既有发行物。

可复现准备命令：

```powershell
gh workflow run check.yml --ref YOUR_CANDIDATE_BRANCH -f release_candidate=true
# Download that run's release-candidate-windows-x64 artifact and verify SHA256SUMS.txt.
gh release create v0.1.0-rc.1 --draft --prerelease --target CANDIDATE_COMMIT --title "BlinkVerdict 0.1.0-rc.1" --notes-file docs/releases/0.1.0-rc.1-notes.md PATH_TO_ZIP PATH_TO_SHA256SUMS
```

若本地连接不能上传附件，可在草稿已存在时通过仓库令牌归档已验证的产物，无需重跑测试：

```powershell
gh workflow run check.yml --ref YOUR_CANDIDATE_BRANCH -f upload_only=true -f source_run=SUCCESSFUL_CANDIDATE_RUN_ID
```

该路径重新核对 ZIP / 内部摘要、版本、架构及草稿目标提交，只向匹配的现有 draft prerelease 添加附件，不创建或公开发行，也不覆盖同名附件。发行结果仍以验收记录和源构建为准。

## 稳定版验收

逐项完成 [测试清单](../testing.md)与真实 Windows 10/11、模型和游戏验证，记录环境和异常。未完成的项目不能勾选。源码、包、验证记录必须对应同一候选；若修改运行时代码，重建候选并重跑受影响检查。

完成稳定验收后再移除 SemVer 的 `-rc.N`、更新锁文件与发布说明。当前打包脚本只允许候选版本，避免未验收源码被误标为稳定版。

## README 编排参考

本轮查阅 [Archify](https://github.com/tt-a1i/archify)、[Flow Launcher](https://github.com/Flow-Launcher/Flow.Launcher)、[UniGetUI](https://github.com/Devolutions/UniGetUI)、[ShareX](https://github.com/ShareX/ShareX)的实际 README：借鉴明确的产品定位、可见的下载入口、快速上手、图示、常见问题与许可/贡献入口。没有复制其产品文案、素材、下载量或兼容性声明。
