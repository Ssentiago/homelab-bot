#![allow(dead_code)]

use crate::ui::screens::Screen;

/// Результат обработки — что FSM должен сделать после возврата из модуля
#[derive(Debug)]
pub enum Action {
    EditUI { screen: Screen },
    SendMessage { text: String, thread: ThreadTarget },
    DeleteMessages(Vec<i64>),
    SwitchMode(Mode),
    Noop,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub enum ModuleError {
    Processing(String),
    Fatal(String),
}
