# StreamixKit

面向播放器开发者的 SXAPI v1 客户端资料与 Rust SDK。

![Stage](https://img.shields.io/badge/stage-source_candidate-orange)
![Protocol](https://img.shields.io/badge/protocol-SXAPI_v1-blue)
![License](https://img.shields.io/badge/license-commercial_proprietary-lightgrey)

**当前版本：0.1.0-rc.1 · Build 20261029.003。尚未发布、尚未通过 SDK 服务器验收。**

规范已绑定服务器源码 `29f600eb47f0f2f893039b654e3b9f7d782576d7`，最低服务器候选版本为 **0.9.0-rc.9**，见 [SOURCE.json](spec/SOURCE.json)。这只是契约来源绑定，不代表 SDK CI、联调或正式发布通过。当前代码要求新的请求头取流契约；不能用于旧的 URL 票据接口。能力声明必须包含 `features.playback_ticket_header=true` 和 `features.credentials_in_urls=false`；协议版本 `1.0` 单独不能证明支持。

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
