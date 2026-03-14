# bot-terminal

一个用 Rust 编写的 Telegram Bot 项目骨架，目标是在聊天窗口里安全地触发本机 terminal 命令。

当前版本重点是先把基础边界和扩展结构搭好：

- 先支持 Telegram，后续如果要接 Discord、Slack 或网页聊天界面，可以在 `platform/` 下继续扩展。
- 命令执行单独放在 `terminal/` 层，方便后面补白名单、审计日志、任务队列和沙箱能力。
- 默认要求配置 Telegram 用户白名单，避免变成一个完全公开的远程 shell。

## 当前能力

- 支持 Telegram 长轮询启动 Bot
- 支持 `/start`、`/help`、`/run <command>`
- 支持按 Telegram 用户 ID 做访问控制
- 支持配置命令执行工作目录
- 支持执行超时和输出截断

## 快速开始

1. 复制环境变量模板：

```bash
cp .env.example .env
```

2. 修改 `.env`，至少填入：

- `TELEGRAM_BOT_TOKEN`
- `BOT_ALLOWED_USER_IDS`

不会创建 Telegram Bot 的话，可先看：[创建并配置 Telegram Bot](docs/telegram-bot-setup.md)

3. 启动：

```bash
cargo run
```

4. 在 Telegram 中发送：

```text
/help
/run pwd
/run ls -la
```

## Docker

构建镜像：

```bash
docker build -t bot-terminal .
```

使用 compose：

```bash
cp docker-compose.yml.example docker-compose.yml
docker compose up -d --build
```

说明：

- Compose 会读取当前目录的 `.env`
- 项目目录会挂载到容器内的 `/workspace`
- Bot 执行命令时默认工作目录是 `/workspace`

## 目录结构

```text
src/
├── app.rs
├── config.rs
├── main.rs
├── platform/
│   ├── mod.rs
│   └── telegram.rs
└── terminal.rs
```

## 简单流程

1. 用户在 Telegram 中发送 `/run <command>`
2. Telegram 平台层接收消息并提取用户 ID、命令文本
3. 访问控制校验该用户是否在白名单中
4. `terminal` 层在受控工作目录中执行命令
5. 收集退出码、标准输出、标准错误
6. 截断过长输出并回发到 Telegram

## 下一步建议

优先做这几件事：

1. 把“任意 shell 命令”改成“命令模板/白名单子命令”
2. 给执行记录加审计日志和请求 ID
3. 把长任务改成后台任务，支持查询状态和取消
4. 给平台层抽象一个统一接口，为以后支持别的平台留口子
5. 增加 Docker 部署和 systemd 示例

## License

MIT
