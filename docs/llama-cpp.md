# 用 llama.cpp 获取答案

BlinkVerdict 连接你已经运行的 **llama-server**。无需额外推理引擎，也不把模型装进常驻程序；仅在按下快捷键时发送请求。`llama-cli` 的交互终端不是 HTTP 服务，需要改用同一项目提供的 `llama-server`。

## 已经启动了服务

在“模型 → 使用服务”选择 **llama.cpp · 本机**，然后填写：

| 设置 | 填写内容 |
|---|---|
| 基础地址 | 默认 `http://127.0.0.1:8080/v1`；按实际端口修改，不重复填写 `/chat/completions` |
| 模型名称 | 服务实际的模型 ID 或 `--alias`，预设为 `blink-local` |
| 密钥 | 未开启认证时留空；开启认证时填写服务设置的密钥 |
| 屏幕输入 | 文字模型选本机 OCR；有视觉能力并加载了匹配投影文件时可选截图 |

设定目标与选项，保存后按 **Ctrl + Alt + Space**。OCR 需要 Windows 已安装相应语言包，不能理解图形布局；截图输入需要视觉模型支持。

如果不知道模型 ID，可查看服务启动信息，或手动打开 `http://127.0.0.1:8080/v1/models`。有认证时该接口同样需要认证；不要把密钥放进网址。应用不会在后台轮询模型列表。

## 从 GGUF 启动服务

下面是 Windows PowerShell 的最小示例。替换模型路径，在 llama.cpp 程序所在目录执行，并保持终端运行：

```powershell
.\llama-server.exe -m "C:\Models\model.gguf" --alias blink-local --host 127.0.0.1 --port 8080
```

选择适合自己设备、支持对话的模型。GPU 层数、上下文长度与线程数请按模型和 llama.cpp 官方说明配置，本应用不改动这些参数。

视觉模型还需要与模型匹配的多模态投影文件，例如：

```powershell
.\llama-server.exe -m "C:\Models\vision-model.gguf" --mmproj "C:\Models\mmproj.gguf" --alias blink-local --host 127.0.0.1 --port 8080
```

仅提供 `--mmproj` 不会让文字模型获得视觉能力；具体架构支持以使用的 llama.cpp 版本为准。服务就绪后再触发决策。

## 连续使用与资源占用

模型加载、休眠、显存与内存由 llama-server 管理。BlinkVerdict 不发送 Ollama 的 `keep_alive` 参数，也不在每次决策后卸载模型。需要更快的连续响应时，保持服务与模型就绪；需要释放资源时，通过服务自身的配置或退出服务处理。

请求默认最多等待 15 秒。首次加载、较长屏幕文字或视觉处理可能超时，可在“连接选项”适当增加等待时间（1–120 秒）和最大输出（64–8192 tokens）。推理模型可能把输出预算用于思考，需为最终答案留出空间。没有固定的响应时长保证。

## 输出方式与排查

该预设使用非流式 Chat Completions，以及 llama-server 的 `response_format: {"type":"json_object","schema":…}`。Schema 只允许 `choice` 为当前选项的稳定 ID（`option_1` 等）或 `null`。程序再校验结果并显示你自定义的文字；约束格式不保证模型判断正确。

- **无法连接**：确认服务就绪、端口一致、Windows 能访问该地址。默认回环 HTTP 不经过系统代理；远程服务需使用 HTTPS。
- **模型不存在**：填写服务实际模型 ID，或用示例中的 `--alias blink-local` 启动。
- **不支持图片**：检查模型与投影文件，或切回本机 OCR。
- **不识别结构化输出参数**：更新 llama.cpp；旧版本或第三方包装服务可尝试保留的“OpenAI 兼容”协议，但不会获得此预设的原生 Schema 约束。
- **已有 OpenAI 兼容配置**：可以继续使用；切到独立的 llama.cpp 协议后，有认证时需重新填写密钥，原配置对应的凭据不会被搬移或删除。

请求格式依据 [llama.cpp server 官方说明](https://github.com/ggml-org/llama.cpp/tree/master/tools/server)和[多模态说明](https://github.com/ggml-org/llama.cpp/tree/master/tools/mtmd)。当前已验证请求构造、结果映射与 Windows 预设保存；使用你的模型权重进行真实推理仍需实机确认，不把协议样例作为模型质量或速度测试。
