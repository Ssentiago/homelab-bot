## [0.3.1] - 2026-07-23

### Bug Fixes
- remove archive features from self_update — jrit uploads raw binary ([`4c8ed14`](https://github.com/Ssentiago/homelab-bot/commit/4c8ed1405e0c3a684c4f73143e0a9700700637be))

## [0.3.0] - 2026-07-23

### Features
- add GET / help endpoint for notify server ([`fea32f1`](https://github.com/Ssentiago/homelab-bot/commit/fea32f1d18c3e8d97d54804487150e4c968e8a72))

## [0.2.0] - 2026-07-23

### Features
- feedback message shows 'Added to' and message count ([`ae11f8f`](https://github.com/Ssentiago/homelab-bot/commit/ae11f8fd0834c0abd9e432aec042ed8ab3536544))

### Bug Fixes
- use asset_identifier for self_update to find correct release asset ([`ce73f4a`](https://github.com/Ssentiago/homelab-bot/commit/ce73f4abbdfa92014009b7cfb4a39eb95c69fd0a))

## [0.1.2] - 2026-07-05

### Bug Fixes
- install rustls ring provider at startup ([`34542a9`](https://github.com/Ssentiago/homelab-bot/commit/34542a99ff61236ae39413d15ca210accad898b4))

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

