# Резюме для дев-агента: Homelab Bot — Forum Topics

## Стек
- Rust, `teloxide` — async фреймворк под tokio для Bot API
- `serde` / `serde_json` — сериализация
- Сборка кросс-компиляцией под линукс через `cross`, релизы — через `jrit` (уже в PATH)

## Forum Topics в приватном чате с ботом

**Настроено в BotFather:**
- Threaded Mode = ON
- Disallow users to create new threads = ON

**Комиссия:** 15% только при реальных Stars-покупках внутри бота. Платных фич нет — не касается.

**Топики создаются один раз, thread_id сохраняется в конфиг/локальную базу**, не пересоздаются при каждом старте:

```rust
use teloxide::prelude::*;
use teloxide::types::ChatId;

async fn create_topic(bot: &Bot, chat_id: ChatId, name: &str) -> ResponseResult<i32> {
    let topic = bot.create_forum_topic(chat_id, name).await?;
    Ok(topic.message_thread_id)
}
```

Отправка в конкретный топик:

```rust
async fn send_to_topic(bot: &Bot, chat_id: ChatId, thread_id: i32, text: &str) -> ResponseResult<()> {
    bot.send_message(chat_id, text)
        .message_thread_id(thread_id)
        .await?;
    Ok(())
}
```

Методы управления: `create_forum_topic`, `edit_forum_topic`, `close_forum_topic`, `delete_forum_topic`.

## Быстрые заметки через сам чат

Юзер пишет текст прямо в топик "Быстрые заметки". Бот фильтрует входящие по `message_thread_id`:

```rust
async fn handle_message(bot: Bot, msg: Message, quick_notes_thread_id: i32) -> ResponseResult<()> {
    if msg.thread_id == Some(quick_notes_thread_id) {
        if let Some(text) = msg.text() {
            // сохранить заметку
        }
    }
    Ok(())
}
```

Топик "Уведомления" — только вывод от бота, юзер туда не пишет.

## Что нужно агенту для старта
- Bot token
- Chat ID (личный)
- Хранение thread_id после первого создания топиков (файл конфига или sqlite/sled)

## Релизы через jrit

Перед началом работы агент должен пройти в README проекта jrit (https://github.com/Ssentiago/jrit) — там описан весь конфиг-формат и логика (`changelog_type`, `release_mode`, `version_files`, `components`).

jrit уже установлен и есть в PATH. Требования для его работы в этом проекте:
- В корне репозитория нужен `jrit.toml`
- `GITHUB_TOKEN` в env, либо авторизованный `gh` CLI
- Файл `CHANGELOG.md` должен физически существовать в репозитории (содержимое может быть пустым), если `changelog_type` не `none`

Конфиг `jrit.toml`:

```toml
[project]
name = "homelab-bot"
repo = "Ssentiago/homelab-bot"
branches = ["main"]

changelog_type = "conventional"
changelog = "CHANGELOG.md"

release_mode = "local"

[[components]]
name = "main"
path = "."
build = "cross build --release --target x86_64-unknown-linux-gnu --no-default-features"
artifact = "./target/x86_64-unknown-linux-gnu/release/homelab-bot"
zip = false

[[components.version_files]]
file = "Cargo.toml"
```

Агент должен:
1. Создать `jrit.toml` с этим содержимым в корне проекта (поправить `repo` под реальное имя репозитория и `name` артефакта под имя бинаря из `Cargo.toml`)
2. Убедиться, что `cross` установлен и таргет `x86_64-unknown-linux-gnu` доступен, иначе `cross build` упадёт
3. Создать физический файл `CHANGELOG.md` в корне репозитория (содержимое может быть пустым)
4. Релиз — простой запуск `jrit` из корня проекта, дальше он сам бампает версию, собирает через `cross`, коммитит, тэгает и публикует на GitHub

## Правила работы и коммитов для агента

- Не смешивать изменения из разных областей/доменов в одном коммите — каждый коммит атомарен и относится к одной логической единице работы (например: отдельно настройка teloxide, отдельно логика топиков, отдельно хранение thread_id, отдельно webhook-обработчик заметок)
- Перед реализацией любой фичи — сначала расписать все шаги, которые потребуются для её реализации, и согласовать план с пользователем до начала кодирования
- После согласования — реализовывать атомарно, коммит за коммитом
- Каждый коммит зависит от предыдущего линейно (без разрозненных параллельных веток логики внутри одной фичи)
- Проект должен компилироваться без ошибок на любом отдельном коммите — не оставлять промежуточные состояния, которые не собираются
- Коммиты — в формате Conventional Commits (`feat:`, `fix:`, `refactor:`), чтобы `changelog_type = "conventional"` в jrit корректно их подхватывал