# BlinkVerdict · 一瞬决断

给眼前的选择，一个答案。

适用于 Windows 10 22H2 / Windows 11 x64；ARM64 与 32 位暂无发行包。

解压后运行 `blink-verdict.exe`。无需安装，无需管理员权限。

1. 首次打开，选择模型服务，填写名称、地址与需要的密钥，点击“下一步”。本地服务需要先自行运行。
2. 写下想判断的事情，以及每行一个的答案，然后点击“完成设置”。
3. 需要时按 Ctrl + Alt + Space。程序会读取当前屏幕，让模型从你的选项中做出选择。云端服务会收到这次画面或 OCR 文字。

单击托盘图标返回设置。右键可暂停或继续决策、切换显示方式。

- “决策”：修改目标、答案与快捷键。点击快捷键，直接按下新的组合键。
- “外观”：选择动画、持续显示、字效与图片；独立调节浮窗背景不透明度。全屏只浮现文字或透明字效，没有背景。颜色、字体与停留时间在“颜色与时长”。
- “位置与大小”：在屏幕上拖动移动、边缘缩放，右键完成；也支持精确输入。
- “预览效果”：短暂查看外观，不读取屏幕、不请求模型、不保存设置。
- “完成”：保存并回到后台。“取消”：放弃本次更改。

API 密钥保存在 Windows 凭据管理器中。程序不默认保存截图或判断历史。

当前为 0.1.0-rc.1 候选版，尚未完成正式版验收。独占全屏游戏可尝试单独安装 Game Bar 组件，兼容性需按游戏确认。

[完整介绍](https://github.com/Amadeus2027/blink-verdict) · [模型连接](https://github.com/Amadeus2027/blink-verdict/blob/main/docs/models.md) · [游戏显示](https://github.com/Amadeus2027/blink-verdict/blob/main/docs/gamebar.md)

配置保存在 `%LOCALAPPDATA%\BlinkVerdict\config.json`，并非与程序同目录。更新时先从托盘退出，再替换程序；保留该目录即可保留设置。移除程序后，可删除该目录，并在 Windows 凭据管理器中删除以 `BlinkVerdict/` 开头的凭据。

包内含 MIT 许可、第三方许可与 `build-info.json`。下载目录的 `SHA256SUMS.txt` 可用 `Get-FileHash -Algorithm SHA256` 核对。程序尚未使用 Authenticode 代码签名，Windows 可能显示未知发布者。
