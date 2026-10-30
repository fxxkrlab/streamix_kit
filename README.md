# StreamixKit

面向播放器开发者的 SXAPI v1 客户端资料与 Rust SDK。

![Stage](https://img.shields.io/badge/stage-source_candidate-orange)
![Protocol](https://img.shields.io/badge/protocol-SXAPI_v1-blue)
![License](https://img.shields.io/badge/license-commercial_proprietary-lightgrey)

**当前版本：0.1.0-rc.1 · Build 20261030.005。规范同步候选，尚未发布。**

规范已绑定服务器源码 `2b069a969f6e0ba8c45bc404671b740c1f057565`，最低服务器候选版本为 **0.9.0-rc.9**，见 [SOURCE.json](spec/SOURCE.json)。该提交仅将实际通过 API 检查的 `2db3276aa025f78145324e5e4b75a07c6033cd6f` 补入审查清单，规范字节相同。SDK 继续要求 `features.playback_ticket_header=true` 和 `features.credentials_in_urls=false`；服务器同时提供 `features.stream_ticket_header=true` 和 `features.stream_query_ticket=false` 两个加法别名。协议版本 `1.0` 单独不能证明支持，不回退到 URL 票据。

原 Build 20261029.004（源码 `c8ee971b6660418bd4cb2836be0286fab76d29b6`）的指定服务器 21 项 Rust 测试与 1 MiB mock CLI 证据保留原身份。本次未改变 Rust 源码或 Cargo 输入，未重新编译 SDK；Build 20261030.005 的规范检查仍待指定服务器执行。来源绑定与旧证据不代表本次真实播放器验收或正式发布。

本仓库不包含服务器源码、部署配置或运行数据。规范由服务器维护，同步完整规范和面向播放器的同源子集，禁止手改副本。

- [五分钟入门](docs/quickstart.md) · [English](README_en.md)
- [服务器连接](docs/connecting.md) · [局域网发现指南](docs/discovery.md) · [客户端请求头](docs/client-request-headers.md)
- [认证](docs/authentication.md) · [凭据请求头](docs/credentials-in-headers.md)
- [浏览](docs/browsing.md) · [播放](docs/playback.md) · [字幕](docs/subtitles.md) · [图片](docs/images.md)
- [错误处理](docs/errors.md) · [版本兼容](docs/versioning.md)
- [Rust SDK](sdks/rust/README.md) · [curl 示例](examples/curl/README.md) · [Rust CLI](examples/rust-cli/README.md)
- [贡献和验证](CONTRIBUTING.md) · [变更记录](CHANGELOG.md)

Rust SDK 提供异步登录、媒体库与影片分页、版本选择、播放会话、短票据续取、单段 Range、进度和结束会话。取流只使用 `X-SXAPI-Stream-Ticket`，字幕使用 `X-SXAPI-Subtitle-Ticket` 加原登录 Bearer；不会把令牌拼入地址。服务器当前只推原文件，不转码。客户端必须如实声明播放能力。

TypeScript、Swift、Kotlin SDK 尚未实现；没有对应软件包或安装入口。

本项目采用[商业专有许可](LICENSE)，公开可读，允许授权的 SDK/API 集成；不是开源许可。SDK 二进制再分发等权利需要单独许可。直接调用 HTTP API 不要求分发 SDK。第三方依赖保留各自许可，见 [NOTICES](NOTICES.md)。

任务：[GitHub #1](https://github.com/fxxkrlab/streamix_kit/issues/1) · [FXX-566](https://linear.app/fxxkrlab/issue/FXX-566)。源码不代表编译、联调、发布或真实播放验收已经完成。
