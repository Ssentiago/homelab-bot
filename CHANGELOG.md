## [0.7.0] - 2026-10-17

### Features
- migrate panel to Rich Messages (Bot API 10.3) ([`50d146b`](https://github.com/Ssentiago/homelab-bot/commit/50d146be6001c4054ce134f2ab6d4a1f8a21a0bf))

### Bug Fixes
- ack every callback immediately, ignore message not modified ([`23efb1e`](https://github.com/Ssentiago/homelab-bot/commit/23efb1e58769bea088fddb423b7f5b75d961cfe1))

## [0.6.0] - 2026-10-11

### Features
- route all git calls in the notes pipeline through crate::auth::run_git ([`6acec52`](https://github.com/Ssentiago/homelab-bot/commit/6acec5202d5cc73d854caf09ce2b696faba4bd86))
- migrate notes to a remote https git repo via inline cred helper ([`9099da5`](https://github.com/Ssentiago/homelab-bot/commit/9099da561183266a13cc6c50702b9645b18d0a4e))

### Bug Fixes
- note rich window shows file title twice and hides empty body ([`6e59d39`](https://github.com/Ssentiago/homelab-bot/commit/6e59d3905540435c66d3cc77ec697c288fe36c4a))

## [0.5.0] - 2026-07-27

### Features
- add file-based logging with tracing-appender ([`e71ccd7`](https://github.com/Ssentiago/homelab-bot/commit/e71ccd7ef1d9626f5312d2dae77aa319620530ac))

### Bug Fixes
- remove extra leading spaces in bot short description ([`8f63066`](https://github.com/Ssentiago/homelab-bot/commit/8f63066be62acfec279bb229fd608b8673b297f2))

### Refactoring
- architecture reform — global FSM, 2 topics, middleware router ([`931c791`](https://github.com/Ssentiago/homelab-bot/commit/931c7917e0b37153b9af71d2284f6df9f9741de2))

## [0.4.1] - 2026-07-25

### Bug Fixes
- replace self_update with self-replace to fix empty binary on update ([`0e12b9a`](https://github.com/Ssentiago/homelab-bot/commit/0e12b9a6859e469e2b653450ccd6bc5336dc4a84))

## [0.4.0] - 2026-07-25

### Features
- replace welcome messages with ModuleHandle-based heartbeat ([`f67a55e`](https://github.com/Ssentiago/homelab-bot/commit/f67a55eb21ee5af9ce23e2c5d0be96925db243ca))
- wire /status command with SQLite stats recording ([`d99c49c`](https://github.com/Ssentiago/homelab-bot/commit/d99c49cde851667f48caf7946f4d7f59de4e2c14))
- add stats module and database table ([`6bc8390`](https://github.com/Ssentiago/homelab-bot/commit/6bc83903f44861c9974d5d5f61fbe1aff51dcb29))
- add dedup module with 5-minute window cache ([`8dc27c3`](https://github.com/Ssentiago/homelab-bot/commit/8dc27c3dfb896ea18a90a9d3ab0147b9b49f153f))
- generic TaskQueue with claim/mark functions ([`f39b554`](https://github.com/Ssentiago/homelab-bot/commit/f39b55421e4b73f24c6c05b54a95b2d1acdab48f))
- integrate queue with notification server - producer inserts only ([`88a2530`](https://github.com/Ssentiago/homelab-bot/commit/88a25301588b6daca29c798aa0ec0d12759bc318))
- SQLite task queue infrastructure for notifications ([`b358bb9`](https://github.com/Ssentiago/homelab-bot/commit/b358bb9c4ec9f5173646de53d72aa182915f3f4a))
- replace dot close with inline keyboard button ([`7cf4643`](https://github.com/Ssentiago/homelab-bot/commit/7cf464368ca88cf2f6f68f2021ca5596d5ca82a0))
- hide ---N--- separators in rich message display ([`8cee5cb`](https://github.com/Ssentiago/homelab-bot/commit/8cee5cbde0bd4e82184e64c0f933619ce3eaee00))
- update rich message on message edit ([`b883267`](https://github.com/Ssentiago/homelab-bot/commit/b883267641319c41952b1f597fb531d8f073e647))
- integrate rich_client into quick notes handler ([`50077cb`](https://github.com/Ssentiago/homelab-bot/commit/50077cbd8d8c155690cad4bbd60e31eb93912bdc))
- rich_client.rs with markdown-based rich messages ([`3fa71a8`](https://github.com/Ssentiago/homelab-bot/commit/3fa71a8898ffd450122bc869caf698ac9ae35061))
- edit messages sync with file via deterministic separators ([`9631470`](https://github.com/Ssentiago/homelab-bot/commit/9631470e20aaef5473ae70c366c58564484ed660))
- deterministic message format helpers with regex ([`a07b63f`](https://github.com/Ssentiago/homelab-bot/commit/a07b63f323dbbd8147ddc03362ae7a2fe8a41eaf))
- dot (.) closes active quick notes window ([`d64f0e9`](https://github.com/Ssentiago/homelab-bot/commit/d64f0e9e5b215ccd953ef5ab7aafce33a54f6906))

### Bug Fixes
- improve notifications health check and heartbeat sync display ([`e4c9d81`](https://github.com/Ssentiago/homelab-bot/commit/e4c9d81e7b84511fbf7e920c5ac06cf0729d0790))
- preserve inline keyboard button during countdown updates ([`d56bc11`](https://github.com/Ssentiago/homelab-bot/commit/d56bc111517263271222cd7de732bd259dab6b81))
- restart countdown on message edit + format separators ([`fcb5b58`](https://github.com/Ssentiago/homelab-bot/commit/fcb5b58678bdacffc591f71b1a8c8f9855b7c70d))
- add double newline between separators and expand tests ([`56c1ede`](https://github.com/Ssentiago/homelab-bot/commit/56c1ede3bb872801d8ecb3129269e7618b9a04e7))
- rich message polish — branch router, strip frontmatter, final heading, format separators ([`3e9d720`](https://github.com/Ssentiago/homelab-bot/commit/3e9d720f4b1d9660161aac28fd9657ad03bbd9ef))

### Refactoring
- central Router with mpsc dispatching ([`ca6fdd5`](https://github.com/Ssentiago/homelab-bot/commit/ca6fdd59f02b3f2bd100335afc6be6e758c17aa7))

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

