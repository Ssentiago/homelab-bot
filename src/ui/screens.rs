//! Экраны панели: rich-рендер + inline-фолбэк.

use frankenstein::rich_message::{
    InputRichBlock, InputRichBlockButtons, InputRichBlockDetails, InputRichBlockList,
    InputRichBlockListItem, InputRichBlockParagraph, InputRichBlockSectionHeading,
    InputRichMessage, RichMessageButton, RichText,
};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// Что сейчас показывает панель. Только отображение — логика живёт в FSM/модулях.
#[derive(Debug, Clone)]
pub enum Screen {
    Main,
    Notes,
    Status {
        chat_id: i64,
        panel_topic: i32,
        monitor_topic: i32,
    },
    ComingSoon,
    History,
    Error {
        message: String,
        fatal: bool,
    },
}

pub fn render(screen: &Screen) -> InputRichMessage {
    let blocks = match screen {
        Screen::Main => {
            let version = env!("CARGO_PKG_VERSION");
            vec![
                heading(&format!("Homelab Bot v{version}"), 1),
                para("Выберите действие:"),
                row(vec![btn_style("📝 Заметки", "menu:notes", "primary"), btn("🎵 Музыка", "menu:music")]),
                row(vec![btn("📊 Саммари", "menu:summary"), btn("📊 Статус", "menu:status")]),
            ]
        }
        Screen::Notes => vec![
            heading("📝 Заметки", 1),
            para("Активно — отправьте сообщение для сохранения"),
            details(
                "Как добавить заметку",
                vec![para("Просто отправьте текст в этот чат. Он сохранится автоматически.")],
            ),
            row(vec![btn("📋 История", "notes:history")]),
            row(vec![btn("← Назад", "menu:main")]),
        ],
        Screen::Status {
            chat_id,
            panel_topic,
            monitor_topic,
        } => {
            let version = env!("CARGO_PKG_VERSION");
            vec![
                heading("📊 Статус", 1),
                list(&[
                    "📝 Заметки — 🟢 активно",
                    "🎵 Музыка — ожидает",
                    "📊 Саммари — ожидает",
                    "📢 Уведомления — 🟢 активно",
                ]),
                details(
                    "О системе",
                    vec![
                        para(&format!("Версия: v{version}")),
                        para(&format!("Chat ID: {chat_id}")),
                        para(&format!("Топик «Панель»: {panel_topic}")),
                        para(&format!("Топик «Мониторинг»: {monitor_topic}")),
                    ],
                ),
                row(vec![btn("← Назад", "menu:main")]),
            ]
        }
        Screen::ComingSoon => vec![
            heading("Скоро", 1),
            para("Coming soon"),
            row(vec![btn("📝 Заметки", "menu:notes"), btn("🎵 Музыка", "menu:music")]),
            row(vec![btn("📊 Саммари", "menu:summary"), btn("📊 Статус", "menu:status")]),
        ],
        Screen::History => vec![
            heading("📋 История", 1),
            para("Пока заглушка"),
            row(vec![btn("← Назад", "menu:main")]),
        ],
        Screen::Error { message, fatal } => {
            let mut b = vec![heading("⚠ Ошибка", 1), para(message)];
            if *fatal {
                b.push(para("Выберите действие:"));
                b.push(row(vec![
                    btn("📝 Заметки", "menu:notes"),
                    btn("🎵 Музыка", "menu:music"),
                ]));
                b.push(row(vec![
                    btn("📊 Саммари", "menu:summary"),
                    btn("📊 Статус", "menu:status"),
                ]));
            } else {
                b.push(row(vec![btn("← Назад", "menu:main")]));
            }
            b
        }
    };
    InputRichMessage::builder().blocks(blocks).build()
}

/// Тот же экран текстом + inline-клавиатура — фолбэк, если rich отвалился.
pub fn fallback(screen: &Screen) -> (String, InlineKeyboardMarkup) {
    match screen {
        Screen::Main => (
            format!("Homelab Bot v{}\n\nВыберите действие:", env!("CARGO_PKG_VERSION")),
            main_menu_keyboard(),
        ),
        Screen::Notes => (
            "📝 Заметки — активно\n\nОтправьте сообщение для сохранения".to_string(),
            notes_keyboard(),
        ),
        Screen::Status {
            chat_id,
            panel_topic,
            monitor_topic,
        } => {
            let version = env!("CARGO_PKG_VERSION");
            (
                format!(
                    "Homelab Bot v{version}\n\n\
                     📝 Заметки — 🟢 активно\n\
                     🎵 Музыка — ожидает\n\
                     📊 Саммари — ожидает\n\
                     📢 Уведомления — 🟢 активно\n\n\
                     Chat ID: {chat_id}\n\
                     Топик «Панель»: {panel_topic}\n\
                     Топик «Мониторинг»: {monitor_topic}",
                ),
                back_keyboard(),
            )
        }
        Screen::ComingSoon => ("Coming soon".to_string(), main_menu_keyboard()),
        Screen::History => ("📋 История (пока заглушка)".to_string(), notes_keyboard()),
        Screen::Error { message, fatal } => {
            if *fatal {
                (
                    format!("⚠ Ошибка модуля: {message}\n\nВыберите действие:"),
                    main_menu_keyboard(),
                )
            } else {
                (format!("⚠ Ошибка: {message}"), back_keyboard())
            }
        }
    }
}

// --- fallback-клавиатуры (прежний вид) ---

fn main_menu_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("📝 Заметки", "menu:notes"),
            InlineKeyboardButton::callback("🎵 Музыка", "menu:music"),
        ],
        vec![
            InlineKeyboardButton::callback("📊 Саммари", "menu:summary"),
            InlineKeyboardButton::callback("📊 Статус", "menu:status"),
        ],
    ])
}

fn back_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback("← Назад", "menu:main")]])
}

fn notes_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("📋 История", "notes:history")],
        vec![InlineKeyboardButton::callback("← Назад", "menu:main")],
    ])
}

// --- rich-хелперы ---

fn rt(s: &str) -> RichText {
    RichText::from(s)
}

fn para(s: &str) -> InputRichBlock {
    InputRichBlock::Paragraph(InputRichBlockParagraph::builder().text(rt(s)).build())
}

fn heading(s: &str, size: u8) -> InputRichBlock {
    InputRichBlock::Heading(InputRichBlockSectionHeading::builder().text(rt(s)).size(size).build())
}

fn row(buttons: Vec<RichMessageButton>) -> InputRichBlock {
    InputRichBlock::Buttons(InputRichBlockButtons::builder().buttons(buttons).build())
}

fn btn(label: &str, data: &str) -> RichMessageButton {
    RichMessageButton::builder().text(rt(label)).callback_data(data).build()
}

fn btn_style(label: &str, data: &str, style: &str) -> RichMessageButton {
    RichMessageButton::builder()
        .text(rt(label))
        .callback_data(data)
        .style(style)
        .build()
}

fn list(items: &[&str]) -> InputRichBlock {
    let items = items
        .iter()
        .map(|s| InputRichBlockListItem::builder().blocks(vec![para(s)]).build())
        .collect();
    InputRichBlock::List(InputRichBlockList::builder().items(items).build())
}

fn details(summary: &str, blocks: Vec<InputRichBlock>) -> InputRichBlock {
    InputRichBlock::Details(
        InputRichBlockDetails::builder().summary(rt(summary)).blocks(blocks).build(),
    )
}
