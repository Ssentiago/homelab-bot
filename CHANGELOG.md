## [0.1.1] - 2026-07-05

### Refactoring
- rename ROOT env var to NOTES_ROOT ([`b7f64ff`](https://github.com/Ssentiago/homelab-bot/commit/b7f64ff854bb7d8a0e1ecd083351d1a3395e0464))

## [0.1.0]

First release of Telegram bot for notes and notifications via Forum Topics.

### Quick Notes

- Write messages in "Quick Notes" topic, bot saves them as markdown files
- Debounce window (default 45s) groups consecutive messages into one file
- Prefix with `!` to force new file: `! Project Ideas` creates `2026-07-04_21-15_Project-Ideas.md`
- Real-time feedback message with countdown timer
- Frontmatter with timestamp and source

### Notifications

- HTTP endpoint `POST /notify` for external services
- Bearer token authorization
- Multipart form support with file attachments
- Async processing — 200 OK immediately, Telegram delivery in background
- Retry logic with exponential backoff (3 attempts)
- Failed notifications logged to `failed_notifications.log`

### Architecture

- Supervised tasks — each component restarts independently on panic
- Async I/O throughout (tokio::fs)
- Structured logging via tracing
- Self-update via `--update` flag (downloads from GitHub releases)

### Configuration

- `.env` for secrets (BOT_TOKEN, CHAT_ID, NOTIFY_TOKEN, ROOT, etc.)
- `config.json` for topic IDs (auto-created on first run)

See [README](README.md) for setup instructions.

