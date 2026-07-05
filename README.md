# Homelab Bot

Telegram bot for notes and notifications in a private chat via Forum Topics.

## Features

- **Quick Notes** — write messages in the "Quick Notes" topic, bot saves them as markdown files
- **Notifications** — external services send HTTP requests, bot forwards them to the "Notifications" topic

## Setup

### 1. Telegram Bot

1. Create a bot via [@BotFather](https://t.me/BotFather)
2. Enable **Threaded Mode** in bot settings
3. Open chat with the bot and press "Start"

### 2. Environment Variables

Copy `.env.example` to `.env` and fill in:

```bash
cp .env.example .env
```

| Variable | Description | How to get |
|----------|-------------|------------|
| `BOT_TOKEN` | Bot token | @BotFather → /mybots → API Token |
| `CHAT_ID` | Private chat ID | [@userinfobot](https://t.me/userinfobot) |
| `NOTES_ROOT` | Folder for notes | Absolute path, e.g. `/home/user/notes` |
| `NOTIFY_SERVER_PORT` | HTTP server port | Default `8787` |
| `NOTIFY_TOKEN` | Token for HTTP requests | Any random string |
| `DEBOUNCE_SECS` | Notes grouping window | Default `45` seconds |

### 3. Run

Download binary from [releases](https://github.com/Ssentiago/homelab-bot/releases), place on server. Create `.env` file next to binary (see "Environment Variables" section), then run:

```bash
./homelab-bot
```

On first run bot automatically creates topics and saves their IDs in `config.json`.

## Usage

### Quick Notes

Write messages in the **"Quick Notes"** topic. Bot groups them by time into markdown files.

**Regular messages:**
```
Buy milk
Tomorrow it will rain
Call mom
```
→ One file: `2026-07-04_21-03.md` with all three messages.

**With explicit title (prefix with `!`):**
```
! Website landing page
Need landing for the project
Deadline — Friday
```
→ File: `2026-07-04_21-15_Website-landing-page.md` (first line with `!` is not included in content).

**Important:**
- If filename already exists, suffix is added: `-2`, `-3`, etc.
- Message with `!` forces a new file, even if previous window is still active

### Notifications

Send HTTP requests to `http://localhost:{NOTIFY_SERVER_PORT}/notify`.

**Example with file:**
```bash
curl -X POST http://localhost:$NOTIFY_SERVER_PORT/notify \
  -H "Authorization: Bearer $NOTIFY_TOKEN" \
  -F "message=Backup completed" \
  -F "level=info" \
  -F "source=backup-script" \
  -F "file=@/path/to/log.txt"
```

**Example without file:**
```bash
curl -X POST http://localhost:$NOTIFY_SERVER_PORT/notify \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $NOTIFY_TOKEN" \
  -d '{"message":"Disk space low","level":"warning","source":"monitoring"}'
```

**Request fields:**

| Field | Required | Description |
|-------|----------|-------------|
| `message` | Yes | Notification text |
| `level` | No | `info` / `warning` / `error` (default `info`) |
| `source` | No | Notification source |
| `file` | No | File attachment |

**Server responses:**
- `200 OK` — request accepted
- `400 Bad Request` — invalid JSON or missing `message`
- `401 Unauthorized` — invalid token

## Topics

On first run bot automatically creates both topics and saves their IDs in `config.json`.

| Topic | Purpose | Who writes |
|-------|---------|------------|
| Quick Notes | Notes | Only you |
| Notifications | Service alerts | Only bot |

## Stop

Press `Ctrl+C` in terminal. Bot shuts down gracefully.
