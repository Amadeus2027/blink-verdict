# 可选 Game Bar 组件（实验性）

此组件用于评估独占全屏的被动显示路径，不是所有游戏通用的覆盖保证。代码已接入主程序，但安装、包构建和游戏行为需要 Windows 实机验证。当前没有已验证游戏。

主程序仍负责截图、OCR 和模型调用。组件只通过本机 Windows AppService 接收结果、样式与用户指定的字效图片，不包含网络权限，也不接收截图、密钥或判断指令。

## 构建与安装

需要 Windows、Visual Studio 2022 的 UWP C# 开发工具、Windows SDK 10.0.19041，以及 NuGet 还原权限。主程序使用绿色版，组件作为 UWP 包单独安装。

1. 在 Visual Studio 打开 `gamebar/BlinkVerdict.GameBar.csproj`，还原依赖，选择 `Debug / x64` 构建并部署到本机。部署需开启 Windows 开发者模式。
2. 或从 Developer PowerShell 执行：

   ```powershell
   msbuild gamebar/BlinkVerdict.GameBar.csproj /restore /p:Configuration=Debug /p:Platform=x64 /p:AppxPackageSigningEnabled=false
   ```

3. 正式分发前使用与 manifest 中 Publisher 匹配的证书签名 Appx/MSIX；不要把私钥或 PFX 提交到仓库。未签名构建不能当作普通可安装发行包。
4. 手动按 `Win+G`，在组件菜单打开 BlinkVerdict，固定组件并开启 Game Bar 的点击穿透；关闭 Game Bar 面板，回到原应用。
5. 获取包家族名：

   ```powershell
   Get-AppxPackage BlinkVerdict.GameBar | Select-Object -ExpandProperty PackageFamilyName
   ```

6. 在主程序“关于 / 状态”页填写包家族名并保存。留空则使用原生通道。

## 行为与限制

- 主程序不会自动打开 Game Bar、激活组件窗口或模拟按键。唯一的窗口激活发生在用户主动从 Game Bar / 开始菜单打开组件时。
- 组件必须已打开、固定、可见、开启点击穿透，并处于 Game Bar 的 PinnedOnly 模式，否则拒绝显示。
- 全屏模式会请求调整组件大小并居中，实际可用尺寸由 Game Bar 决定；拒绝调整时跳过显示。不能保证跨屏位置或覆盖完整物理显示器，须实测记录。
- 截图前通过 IPC 清空上次结果，再执行屏幕捕获；不承诺能捕获独占或受保护画面。截图黑屏会停止判断。
- 自定义 PNG 通过 AppService 消息传入，仅支持不超过 32 KB 的文件；内置三组字效随组件打包，不逐次传图。
- Game Bar 的组件宿主背景、边框、标题或请求透明度可能影响“完全隐身”效果，需要实机确认；通过验收前保持实验性标记。
- AppService 单次连接 / 消息等待上限约 3 秒。模式选定后不自动回退到原生窗口。
- 组件未连接、取消固定或关闭点击穿透时保持内容隐藏；模型请求失败也会清空处理中状态。默认结果到时隐身；保留时长为 0 时需通过下一次请求或手动关闭组件清除。

实现参考微软 [XboxGameBarSamples](https://github.com/microsoft/XboxGameBarSamples) 的公开激活与 manifest 接口。发行前执行 `docs/testing.md`，不要将编译通过等同游戏兼容。
