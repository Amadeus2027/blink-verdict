# 看看答案如何浮现

![BlinkVerdict：一次按键，两种显示方式](assets/showcase.gif)

给眼前的选择，一个答案。

短片以舒缓的镜头展示一次按键、全屏浮现与常驻浮窗，再看一眼属于你的决策与外观设置。README 中保留约八秒的静音循环，完整产品短片约二十四秒。

演示使用软件内置的外观预览，YES 是示例答案，没有向模型发送画面。全屏只显示文字，常驻浮窗使用可调半透明圆角背景；当前不包含背景模糊。

完整短片可在本次录制任务的 `native-ui-showcase-video` 下载项中获取，需要登录 GitHub；下载链接将在录制完成后更新。

<details>
<summary>录制来源与复现方式</summary>

所有界面均来自真实的 Windows Rust 程序。源截图位于 [showcase](assets/showcase/01-decision.png)，[来源记录](assets/showcase/provenance.json) 保存源提交、操作系统、字体、采集方式和检查结果。

采集在独立的临时配置中运行，使用 `--recordable-preview` 禁用决策触发并允许录制浮层；普通启动仍排除自身截图。录制脚本拒绝连接已有的常驻实例，并检查窗口归属。无需真实密钥，不触发截图决策、OCR、模型或 Game Bar。

临时原生窗口提供浅色渐变背景，真实浮层显示在其上。剪辑只加入字幕、裁切、缩放与淡白转场，不在后期伪造玻璃、模糊或应用状态。静态设置镜头与预览序列均为演示画面，不代表游戏兼容或性能验证。

手动运行 **Windows build and core checks** 工作流，可重新采集并导出。日常 push / PR 仍只运行最小核心检查。字体仅安装在录制环境，文档素材不加入程序便携包。

本地需要 PowerShell 7、可用的 Windows 桌面及 GDI。先退出已有实例，在临时桌面会话中运行，并将 `RUNNER_TEMP` 指向临时目录：

```powershell
cargo build --locked --release
./scripts/showcase/capture.ps1
```

编辑阶段需要 FFmpeg 与中文字体：

```bash
python3 scripts/showcase/render.py showcase-frames showcase-edit /path/to/CJK-font.otf
```

</details>

设计参考：[Apple 材质](https://developer.apple.com/design/human-interface-guidelines/materials)、[Things](https://culturedcode.com/things/)、[Raycast](https://www.raycast.com/blog/the-new-raycast)。主视觉以用户提供的 ChatGPT iOS 截图为准。
