# 模型与服务配置

主程序保留 Rust + Win32。模型在你选择的云端或本地服务运行，BlinkVerdict 只在快捷键触发时请求它，不下载权重、不启动模型进程、不定时检查服务。

## 云端专用决策接口

在“模型”中选择“使用服务”，连接信息会自动填入。填写实际模型名称和密钥；需要手动指定协议或等待时间时，打开“连接选项”。

| 预设 | 基础地址 | 默认模型 | 发送格式 |
|---|---|---|---|
| TypeSafe Jev | `https://api.typesafe.ai/v1` | `jev-latest` | `state` + `questions`；图片用 `assets` 引用 |
| Perplexity Decider | `https://api.perplexity.ai/v1` | `pplx-decider-latest` | `state` + `questions`；图片嵌入 state |
| Cloudflare Clef | `https://api.cloudflare.com/client/v4/accounts/YOUR_ACCOUNT_ID/ai/run` | `clef` | `state` + `questions` + 可选 `images` |
| OpenAI Decisions | `https://api.openai.com/v1` | `gpt-6-luna` | `input` + questions 数组 |

Cloudflare 需把 `YOUR_ACCOUNT_ID` 替换为实际 Account ID，使用 Workers AI API Token；程序追加 `/@cf/cloudflare/模型名`。`clef-flash` 等模型名需与账号可用服务匹配。其他基础地址填到版本路径，程序追加 `/systemone` 或 `/decisions`，不要重复填写末尾路由。

Jev 图片输入是厂商实验能力，选截图输入时发送其要求的 `TypeSafe-Danger-Zone-Images: true` 请求头；不支持的版本需手动选择 OCR。Clef 单张截图不得超过 4 MiB，超限会提示，不静默更换输入方式。模型名称 / 可用额度 / 视觉支持会随厂商更新；预设不代表账号已有权限。

这四种专用接口都发送用户的目标和固定选项，额外提供 `uncertain` 作为无法判断选项。结果直接读取结构化 choice，不要求模型先生成解释再解析文字。概率、置信度如有返回，可在托盘状态查看；只在进程内保留最近状态，不写判断历史。不同服务的置信度定义不同，不做统一正确率承诺或强制阈值。

## 本地决策服务

本地模式依然通过 HTTP 请求已启动的服务。无认证时密钥可留空；有认证则填写对应服务的密钥。回环地址绕过系统代理，允许 HTTP；远程地址要求 HTTPS。

| 本地预设 | 实际路由 | 适用服务 |
|---|---|---|
| System One | `http://127.0.0.1:8000/v1/systemone` | vLLM 等的 `state/questions` 协议，图片用顶层 `images` |
| Decider | `http://127.0.0.1:8000/v1/decisions` | 实现 Perplexity 的 map 格式决策协议的服务 |
| OpenAI Decisions（文字） | `http://127.0.0.1:8000/v1/decisions` | 实现 OpenAI 的 array 格式协议的服务；vLLM 当前此路由仅支持文字 |

8000 是可修改的示例端口，`local-decision-model` 是占位模型名。请替换成服务实际启动的模型名 / served-model-name，并按服务能力选择截图或 OCR。本地模型使用 Jev `assets` 图片格式时，选 TypeSafe Jev 协议并修改为本地基础地址。

例如，根据 [vLLM 官方结构化决策文档](https://docs.vllm.ai/en/latest/models/structured_decisions_models/)，在支持的推理环境启动：

```bash
vllm serve Cloudflare/clef-flash --max-model-len 16384
```

随后在 BlinkVerdict 选择“本地决策 · System One”，基础地址填 `http://127.0.0.1:8000/v1`，模型填 `Cloudflare/clef-flash`，根据实际模型选择截图或 OCR。需要自行满足 vLLM 的系统、版本和硬件要求；WSL / 容器需要将服务端口提供给 Windows 本机。此示例来自服务文档，尚未在本项目环境实际加载权重验证。

**同一路由名不代表同一协议**：Perplexity 与 OpenAI 都叫 `/decisions`，但问题和答案格式不同。闭源云端产品也不因此获得可下载权重；“支持本地”指支持兼容协议的本地推理服务。显存、权重和推理内存属于服务本身，不计为主程序常驻占用。

## 常规模型保持可用

OpenAI Chat Completions、Responses、Anthropic、Gemini 和 Ollama 接口全部保留，含 DeepSeek、Qwen、SiliconFlow、OpenRouter 和 LM Studio 预设。模型必须能输出应用要求的严格 JSON；截图要求视觉模型，文字模型请选本机 OCR。

常规接口使用 64–8192 的最大输出 token 设置；专用决策接口不发送该参数，界面会禁用该字段。已有配置中的协议、地址、模型和凭据目标不变；新安装默认 Jev。

## 协议来源与验证范围

- [TypeSafe System One](https://docs.typesafe.ai/api/system-one) · [Jev 图片格式](https://docs.typesafe.ai/images)
- [Perplexity Decisions](https://docs.perplexity.ai/api-reference/decisions-post)
- [Cloudflare Clef](https://developers.cloudflare.com/workers-ai/models/clef/)
- [OpenAI Decisions](https://developers.openai.com/api/docs/guides/decisions)
- [vLLM 本地结构化决策](https://docs.vllm.ai/en/latest/models/structured_decisions_models/)

协议实现依据以上官方文档，并使用合成样例验证请求 / 答案映射。真实服务、账号授权、模型质量与本地权重运行仍需发行前验证；没有使用用户密钥进行在线测试。
