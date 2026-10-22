# BlinkVerdict · 一瞬决断

Windows 10/11 开发预览；保留 Rust 原生实现。解压后运行 `blink-verdict.exe`，从托盘进入设置，无需管理员权限。内置字效已嵌入 EXE，不需要其他素材文件。

1. “模型”页选择并填入服务预设，填写实际模型名称、地址、所需密钥。支持云端决策 API、本地决策服务以及常规视觉 / 文字模型；本地服务需要自行运行。
2. “决策”页填写判断目标和答案选项，默认 YES / NO。选项之外仍需要明确目标。
3. 默认按 `Ctrl + Alt + Space` 截取当前屏幕并触发一次决策；可在设置修改快捷键。云端服务会接收截图或 OCR 文字。
4. 全屏浮现 / 悬浮窗口可切换；“外观”“字效”页调整字体、配色、持续时间与动画。“浮窗”页填写位置、大小，或从托盘编辑后拖动 / 缩放；再次点击编辑项恢复点击穿透。悬浮时长为 0 时保持显示。
5. 错误、最近结果和决策模型返回的概率 / 置信度，从托盘“查看状态 / 最近错误”读取。

- [完整说明与源码](https://github.com/Amadeus2027/blink-verdict)
- [模型协议与本地配置](https://github.com/Amadeus2027/blink-verdict/blob/main/docs/models.md)
- [可选 Game Bar 组件](https://github.com/Amadeus2027/blink-verdict/blob/main/docs/gamebar.md)
- [测试与已知限制](https://github.com/Amadeus2027/blink-verdict/blob/main/docs/testing.md)

当前为开发预览，真实模型服务、游戏焦点 / 输入和内存占用仍需 Windows 实机验收。独占全屏游戏不能保证显示；不使用注入或图形 Hook。
