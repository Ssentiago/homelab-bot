#![allow(dead_code)]

use teloxide::types::InlineKeyboardMarkup;

/// Результат обработки — что FSM должен сделать после возврата из модуля
pub enum Action {
    EditUI { text: String, keyboard: InlineKeyboardMarkup },
    SendMessage { text: String, thread: ThreadTarget },
    DeleteMessages(Vec<i64>),
    SwitchMode(Mode),
    Noop,
}

pub enum ThreadTarget {
    Input,
    Output,
}

/// Классификация апдейта — результат работы мидлвари
#[derive(Debug, Clone)]
pub enum RouteKind {
    MenuNavigation { target: Mode },
    ModuleInput { payload: InputPayload },
}

#[derive(Debug, Clone)]
pub enum InputPayload {
    Text(String),
    Edit(String),
    Callback(String),
    RawMessage(Box<teloxide::types::Message>),
    RawCallback(Box<teloxide::types::CallbackQuery>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    MainMenu,
    Notes,
    Music,
    Summary,
    Status,
}

pub enum ModuleError {
    Processing(String),
    Fatal(String),
}
