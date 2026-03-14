# 创建并配置 Telegram Bot

这份文档补充说明如何拿到下面两个必须的配置：

- `TELEGRAM_BOT_TOKEN`
- `BOT_ALLOWED_USER_IDS`

如果你只是想让自己能使用这个 Bot，按本文一步步操作即可。

## 1. 用 BotFather 创建 Bot

Telegram 官方提供了一个叫 `@BotFather` 的机器人，用来创建和管理其他 Bot。

操作步骤：

1. 打开 Telegram，搜索 `@BotFather`
2. 进入聊天后，发送 `/start`
3. 发送 `/newbot`
4. 按提示输入你的 Bot 名称

名称是展示给别人看的，可以是中文或英文，例如：

```text
My Terminal Bot
```

5. 再按提示输入 Bot 的用户名

用户名需要满足：

- 必须是唯一的
- 必须以 `bot` 结尾
- 一般只能包含英文字母、数字和下划线

例如：

```text
my_terminal_helper_bot
```

创建成功后，`@BotFather` 会返回一段类似下面格式的 token：

```text
1234567890:AAExampleReplaceThisWithRealToken
```

把它填到 `.env` 里的：

```env
TELEGRAM_BOT_TOKEN=1234567890:AAExampleReplaceThisWithRealToken
```

注意：

- 这个 token 相当于 Bot 密码，不要提交到 Git 仓库，也不要发给别人
- 如果 token 泄露了，可以回到 `@BotFather` 用 `/revoke` 重新生成

## 2. 获取你自己的 Telegram 用户 ID

这个项目默认按 Telegram 用户 ID 做白名单控制。只有在 `BOT_ALLOWED_USER_IDS` 里的用户，才能执行 `/run` 命令。

拿自己用户 ID 的简单方式：

1. 在 Telegram 里搜索一个能返回用户信息的机器人，例如 `@userinfobot`
2. 打开后发送 `/start`
3. 它会回复你的用户 ID

拿到以后，填入 `.env`：

```env
BOT_ALLOWED_USER_IDS=123456789
```

如果要允许多个人使用，用英文逗号分隔：

```env
BOT_ALLOWED_USER_IDS=123456789,987654321
```

注意：

- 这里填的是数字 ID，不是用户名，不是手机号
- 这个项目会把 `BOT_ALLOWED_USER_IDS` 按逗号拆分
- 如果这里没填你的 ID，Bot 可以启动，但你发 `/run` 时会被拒绝

## 3. 推荐的 `.env` 示例

可以参考下面的最小配置：

```env
TELEGRAM_BOT_TOKEN=1234567890:AAExampleReplaceThisWithRealToken
BOT_ALLOWED_USER_IDS=123456789
BOT_WORKDIR=.
BOT_COMMAND_TIMEOUT_SECS=20
BOT_MAX_OUTPUT_CHARS=3000
```

几个字段的含义：

- `TELEGRAM_BOT_TOKEN`：BotFather 返回的 token
- `BOT_ALLOWED_USER_IDS`：允许执行命令的 Telegram 用户 ID 列表
- `BOT_WORKDIR`：Bot 执行命令时所在目录
- `BOT_COMMAND_TIMEOUT_SECS`：单条命令超时时间，单位秒
- `BOT_MAX_OUTPUT_CHARS`：单次返回的最大字符数

## 4. 第一次和 Bot 对话

创建完成后，还需要你主动给自己的 Bot 发一条消息，不然它没法和你开始聊天。

步骤：

1. 在 Telegram 搜索你刚创建的 Bot 用户名
2. 打开聊天窗口
3. 点击 `Start`，或者手动发送 `/start`

如果项目已经运行起来，再发送：

```text
/help
```

然后试一条简单命令：

```text
/run pwd
```

## 5. 常见问题

### 找不到自己创建的 Bot

先确认你在 `@BotFather` 那里设置的用户名是否真的创建成功了，并且用户名是以 `bot` 结尾的。

### `/run` 提示没有权限

通常是 `BOT_ALLOWED_USER_IDS` 没填对。优先检查：

- 填的是不是纯数字用户 ID
- 有没有把多个 ID 用英文逗号分隔
- 改完 `.env` 后有没有重启程序

### token 填了还是启动失败

优先检查：

- token 有没有多复制空格
- `.env` 文件是否放在项目根目录
- 是否把示例 token 直接拿来用了

### 想换一个新的 token

去 `@BotFather` 使用相关管理命令重新生成 token，然后同步更新 `.env` 并重启服务。
