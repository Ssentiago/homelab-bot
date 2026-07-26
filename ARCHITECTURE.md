# ARCHITECTURE.md — Homelab Bot

## Overview

Telegram-бот на Rust, работающий в приватном чате с Forum Topics. Центральный FSM (finite state machine) управляет модулями через единую persistent UI-панель. Два постоянных топика — «Панель» (юзер) и «Мониторинг» (система).

## Конфигурация

### .env (секреты, не коммитится)

| Переменная | Назначение |
|---|---|
| `BOT_TOKEN` | Токен бота |
| `CHAT_ID` | ID приватного чата |
| `NOTES_ROOT` | Абсолютный путь для быстрых заметок (`.md`) |
| `NOTIFY_SERVER_PORT` | Порт HTTP-сервера (дефолт `8787`) |
| `NOTIFY_TOKEN` | Токен авторизации для `/notify` |
| `DEBOUNCE_SECS` | Таймер дебаунса заметок (дефолт `45`) |

### config.json (коммитится)

```json
{
    "input": 12345,
    "output": 12346
}
```

Создаётся автоматически при первом запуске. `input` — топик «Панель», `output` — топик «Мониторинг».

## Архитектура запуска

Бот запускает 5 tokio-задач:

```
main.rs
├── task: Router (teloxide Dispatcher)
│   ├── /start command → Fsm::reset_to_main_menu
│   ├── messages/edits/callbacks → classify_* → RouteKind → Fsm::handle_routed
│   └── Только топик «Панель» (фильтр по input thread_id), остальное игнорируется
│
├── task: HTTP-сервер (axum, supervised)
│   └── /notify → queue → worker → «Мониторинг»
│
├── task: Notification worker
│   └── забирает задачи из SQLite-очереди, отправляет в телеграм с retry
│
├── task: Heartbeat updater
│   └── обновляет описание чата статусом модулей
│
└── task: Stats cleanup
    └── периодически чистит старую статистику
```

Общее состояние через `Arc`:
- `Arc<Config>` — конфиг
- `Arc<SqlitePool>` — SQLite пул
- `Arc<Heartbeat>` — статус модулей
- `Arc<tokio::sync::Mutex<Fsm>>` — центральный FSM (общий для router и HTTP-сервера)

## Центральный FSM

`src/fsm.rs` — единая точка диспатча для топика «Панель».

### Mode (состояния)

```rust
enum Mode {
    MainMenu,   // Главное меню с клавиатурой
    Notes,      // Быстрые заметки
    Status,     // Статус бота и модулей
    Music,      // (placeholder)
    Summary,    // (placeholder)
}
```

### Поток обработки

```
Telegram Update из топика «Панель»
  │
  ├─ /start → filter_command → Fsm::reset_to_main_menu()
  │
  ├─ message/edited_message → classify_message → RouteKind::ModuleInput(RawMessage)
  └─ callback_query → classify_callback
        ├─ "menu:*" → RouteKind::MenuNavigation(target)
        └─ другое  → RouteKind::ModuleInput(RawCallback)
            │
            ▼
Fsm::handle_routed(route, chat_id, msg_id)
  │
  ├─ MenuNavigation → switch_mode_from → Action::EditUI (обновление панели)
  └─ ModuleInput → handle_module_input(payload)
        ├─ RawMessage → Notes: mpsc → handler::run
        │              MainMenu/Status: удаляется, Noop
        └─ RawCallback → Notes: "notes:history" → EditUI
                          остальное → callback_tx → handler::run (close_window)
            │
            ▼
Fsm::execute_actions — применяет Action:
  EditUI, SendMessage, DeleteMessages, SwitchMode, Noop
```

### Persistent UI Panel (`src/ui.rs`)

Одно редактируемое сообщение в топике «Панель». `message_id` хранится в SQLite (`persistent_ui`) для восстановления после перезапуска. При старте пытается `editMessageText` существующего сообщения; если не удалось — `sendMessage` нового.

- `edit()` — редактирует in-place; при `MESSAGE_NOT_MODIFIED` молча пропускает (без дублирования)
- `force_resend()` — удаляет старую панель, отправляет новую (используется `/start`)
- `main_menu_keyboard()` — 2×2 кнопки: Заметки/Музыка/Саммари/Статус

### Навигация

Всё через inline-кнопки, команд нет. `/status` удалён, информация доступна через кнопку «📊 Статус». `/start` — сброс FSM в MainMenu с переотправкой панели.

## Модули

### 1. Быстрые заметки

Активируются кнопкой «📝 Заметки» в главном меню. Пользователь пишет текст в топик «Панель» — бот сохраняет как `.md` файлы в `NOTES_ROOT`.

Архитектура: `NotesModule` (`src/modules/quick_notes/notes_module.rs`) — мост между FSM и handler'ом. Спавнит `handler::run()` как фоновую задачу с mpsc-каналами, форвардит входящие сообщения и колбэки.

При выходе из режима (← Назад или `/start`):
- Активное окно с таймером — абортится и удаляется
- Закрытые окна — батч-удаляются из `pending_cleanup` (SQLite)
- Все сообщения юзера за сессию — удаляются

#### Подмодули

- `handler.rs` — логика окон, запись на диск, `extract_explicit_title`, `slugify`, `truncate_at_word_boundary`
- `format.rs` — `replace_message()`, `append_message()`, `find_message()` (поиск по `---N---`)
- `rich_client.rs` — `RichClient`: обёртка над frankenstein для rich-сообщений с обратным отсчётом

### 2. Уведомления

Топик «Мониторинг». Только вывод от бота, пользователь туда не пишет.

HTTP-эндпоинт `POST /notify` (multipart/form-data, авторизация Bearer). Нотификации попадают в SQLite-очередь, фоновый воркер отправляет с дедупликацией (5-минутное окно) и retry (до 5 попыток).

### 3. Музыка / Саммари

Placeholder-кнопки в меню. При нажатии — «Coming soon».

## Middleware Router (`src/router.rs`)

Чистые функции `classify_message`/`classify_callback` фильтруют по `thread_id == input` и преобразуют в `RouteKind`. dptree-диспетчер обрабатывает четыре ветки:

- `filter_command::<Command>()` — `/start`
- `filter_message()` — сообщения
- `filter_edited_message()` — правки
- `filter_callback_query()` — колбэки

## База данных (`src/db.rs`)

SQLite через `sqlx` с WAL-режимом. Пять таблиц:

| Таблица | Назначение |
|---|---|
| `pending_notifications` | Очередь уведомлений (id, status, attempts, retry) |
| `stats` | Дневная статистика (kind, timestamp) |
| `persistent_ui` | ID UI-панели для восстановления |
| `action_history` | Аудит действий (модуль, action, content, TTL 1 день) |
| `pending_cleanup` | ID rich-сообщений для удаления при выходе из Notes |

## Логирование

Файловый логер: `/var/log/homelab-bot/bot.log` с ежедневной ротацией (`tracing-appender`). `#[instrument]` на ключевых методах FSM, UI, NotesModule.

## Самообновление

`--update` → проверяет GitHub Releases → скачивает новый бинарник → `self_replace::self_replace()` → атомарная замена, systemd перезапускает.
