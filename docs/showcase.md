# 一次完整体验

![BlinkVerdict：完整功能与运镜预览](assets/showcase.gif)

从一次按键，到属于你的答案。GIF 包含全部主要章节；[完整 39 秒产品短片 · MP4](assets/showcase.mp4) 保留 1280 × 800、24 帧/秒的流畅运镜。GIF 约 1 MB，MP4 约 1.9 MB。两种文件均保存在仓库，无需下载构建附件。

| 时间 | 看到什么 |
|---|---|
| 00:00–00:04 | 界面主镜头：侧向透视逐渐转向正面 |
| 00:04–00:08 | 快捷键与全屏文字，没有背景遮罩 |
| 00:08–00:12 | 常驻浮窗与独立背景透明度 |
| 00:12–00:16 | 自定义目标、选项和快捷键 |
| 00:16–00:20 | 浮窗位置、大小与精确调整 |
| 00:20–00:24 | 出现动画、透明度、字号和时长 |
| 00:24–00:30 | 清晰纯色、流光玻璃、柔彩手写的原生预览 |
| 00:30–00:36 | 云端/API 与本机模型连接 |
| 00:36–00:39 | 新的产品标识与结尾主镜头 |

所有界面来自真实 Windows 程序，使用独立演示配置。YES 是外观预览中的示例答案，演示没有向模型发送画面、使用真实密钥或运行本地模型。

<details>
<summary>录制来源、镜头与复现方式</summary>

[录制与导出任务](https://github.com/Amadeus2027/blink-verdict/actions/runs/39955205509) · [来源与后台采样记录](assets/showcase/provenance.json) · [原生设置截图](assets/showcase/01-decision.png)

窗口、全屏文字、常驻浮窗、三种字效和本机服务配置均由 `--recordable-preview` 模式中的原生程序绘制。普通启动仍排除自身截图，录制模式禁用决策触发。临时窗口只提供演示桌面背景。

镜头将原生截图作为平面进行透视推进与侧向移动，并加入分层阴影、缓入缓出、字幕和转场。这些是影片的剪辑表达，不是软件自身的立体界面或实时玻璃效果。未伪造模型答案、配置连接状态或应用功能。GIF 加速至约 26 秒、降低分辨率与帧率，但保留全部章节；MP4 适合查看细节与完整运镜。

录像结尾关闭设置和预览，用现有 `measure-idle.ps1` 进行 10 次短采样。CIM 查询使实际区间约为 15 秒，结果写入来源记录。它是开发验证，不替代发行前的 60 秒资源验收。

在 Windows PowerShell 7 和临时桌面会话中，退出已有实例后运行：

```powershell
cargo build --locked --release
./scripts/showcase/capture.ps1
```

编辑需要 FFmpeg、Pillow 和中文字体，它们只用于文档制作，不随程序分发：

```bash
python3 -m pip install 'Pillow>=11,<14'
python3 scripts/showcase/render.py showcase-frames showcase-edit /path/to/CJK-font.otf
```

设计参考：[Linear](https://linear.app/)、[Screen Studio](https://screen.studio/) 的产品演示，以及 [三不五时与影视飓风合作的产品长镜头幕后](https://www.bilibili.com/video/BV1Uc411w77x/) 对仰视、环绕、前后景与速度变化的讲解。此处用平面透视实现适合软件界面的运镜，没有复制其素材或声称具备同等实拍制作。

</details>
