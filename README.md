# StreamixKit

面向播放器开发者的 SXAPI v1 客户端资料与 Rust SDK。

![Stage](https://img.shields.io/badge/stage-source_candidate-orange)
![Protocol](https://img.shields.io/badge/protocol-SXAPI_v1-blue)
![License](https://img.shields.io/badge/license-commercial_proprietary-lightgrey)

**当前版本：0.1.0-rc.2 · Build 20261111.007。规范同步候选，尚未发布。**

规范已绑定服务器源码 `205a05937ea19b390e62770f87e126b3fec4578a`（0.9.0-rc.11 候选，尚未正式发布），最低服务器候选版本仍为 **0.9.0-rc.9**，见 [SOURCE.json](spec/SOURCE.json)。RC11 规范相对 RC10 只增不删：播放器子集从 70 个操作增至 77 个（新增 `GET /users/me/resume`、`GET /users/me/next-up`、`GET /media/initials`，以及 `POST /sessions/current/logout`、`POST /playback/sessions/{id}/end`、`POST /account/credentials/{id}/revoke`、`POST /users/me/streams/{domain}/{id}/terminate` 四个 POST 写法），能力声明新增 `features.list_user_state`、`features.watch_queues`、`features.media_initials`、`features.subtitle_match`，列表和详情新增 `user_state`、`external_ids.tvdb`、`has_logo`，字幕新增 `match`、`other_version`，`/media` 新增外部编号、首字母、类型多选、继续观看筛选、更多排序和 `include_total`，JSON 读取支持 ETag/304。唯一影响 SDK 类型的是 `GET /media` 可带 `include_total=false`，此时 `total` 为 null，所以 `Page.total` 改为 `Option<i64>`；SDK 自己的请求不带这个参数，服务器仍返回数字。SDK 不依赖其他新接口，最低服务器候选版本仍为 **0.9.0-rc.9**。SDK 继续要求 `features.playback_ticket_header=true` 和 `features.credentials_in_urls=false`；服务器同时提供 `features.stream_ticket_header=true` 和 `features.stream_query_ticket=false` 两个加法别名。协议版本 `1.0` 单独不能证明支持，不回退到 URL 票据。

Build 20261111.007 改了 Rust 源码（`Page.total`）并新增一项测试，SDK 版本升到 0.1.0-rc.2。本机工作站上 22 项 Rust 测试和严格 Clippy 通过，规范、类型契约、凭据和链接静态检查通过；指定服务器检查、真实播放器验收和发布都未执行。原 Build 20261029.004（源码 `c8ee971b6660418bd4cb2836be0286fab76d29b6`）的指定服务器 21 项测试与 1 MiB mock CLI 证据只属于那次源码。

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
