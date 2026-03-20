use std::time::Duration;

use poppingboba::spinner::{Spinner, SpinnerType};
use ratatui_core::style::Stylize;
use tuirealm::{
    Application, Component, Event, EventListenerCfg, MockComponent, NoUserEvent, PollStrategy,
    Update,
    command::Cmd,
    event::{Key, KeyEvent, KeyModifiers},
    terminal::{TerminalAdapter, TerminalBridge},
};

pub const GLOBAL_FPS: u32 = 60;

#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    NextSpinner,
    PreviousSpinner,
    Tick,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Id {
    Spinner,
    Label,
}

#[derive(MockComponent)]
struct MySpinner {
    component: Spinner,
}

impl Component<Msg, NoUserEvent> for MySpinner {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::AppClose),
            Event::Keyboard(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => Cmd::Tick,
            Event::Tick => Cmd::Tick,
            Event::Keyboard(KeyEvent {
                code: Key::Char('j'),
                modifiers: KeyModifiers::NONE,
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::NextSpinner),
            Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::NONE,
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::PreviousSpinner),
            _ => Cmd::None,
        };

        match self.perform(cmd) {
            tuirealm::command::CmdResult::Changed(..) => Some(Msg::Tick),
            _ => None,
        }
    }
}

impl MySpinner {
    pub fn new(id: usize) -> Self {
        Self {
            component: Spinner::new(SPINNERS[id], GLOBAL_FPS),
        }
    }
}

pub struct Model<T: TerminalAdapter> {
    pub app: Application<Id, Msg, NoUserEvent>,
    pub quit: bool,
    pub redraw: bool,
    pub terminal: TerminalBridge<T>,
    pub spinner_id: usize,
}

impl<T: TerminalAdapter> Model<T> {
    pub fn view(&mut self) {
        // dbg!("called view on model");
        self.terminal
            .draw(|frame| {
                // dbg!("called draw on term");
                // let chunks = Layout::default()
                //     .direction(Direction::Horizontal)
                //     .margin(1)
                //     .constraints([Constraint::Length(3), Constraint::Fill(1)])
                //     .split(frame.area());

                // app.view(&Id::Spinner, frame, chunks[0]);
                // app.view(&Id::Label, frame, chunks[1]);
                self.app.view(&Id::Spinner, frame, frame.area());
            })
            .expect("to render");
    }
}

const SPINNERS: [SpinnerType; 9] = [
    SpinnerType::dot(),
    SpinnerType::mini_dot(),
    SpinnerType::line(),
    SpinnerType::pulse(),
    SpinnerType::ellipsis(),
    SpinnerType::jump(),
    SpinnerType::globe(),
    SpinnerType::meter(),
    SpinnerType::moon(),
];

impl<T: TerminalAdapter> Update<Msg> for Model<T> {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        let msg = msg?;

        self.redraw = true;
        match msg {
            Msg::AppClose => {
                self.quit = true;
                None
            }
            Msg::NextSpinner => {
                self.spinner_id = (self.spinner_id + 1) % SPINNERS.len();
                self.app
                    .remount(
                        Id::Spinner,
                        Box::new(MySpinner::new(self.spinner_id)),
                        Default::default(),
                    )
                    .expect("remount to work");
                None
            }
            Msg::PreviousSpinner => {
                self.spinner_id = if self.spinner_id == 0 {
                    SPINNERS.len() - 1
                } else {
                    self.spinner_id - 1
                };
                self.app
                    .remount(
                        Id::Spinner,
                        Box::new(MySpinner::new(self.spinner_id)),
                        Default::default(),
                    )
                    .expect("remount to work");
                None
            }
            _ => None,
        }
    }
}

fn main() {
    let mut app: Application<Id, Msg, NoUserEvent> = Application::init(
        EventListenerCfg::default()
            .crossterm_input_listener(Duration::from_millis(20), 2)
            .tick_interval(Duration::from_secs(1) / GLOBAL_FPS),
    );

    app.mount(Id::Spinner, Box::new(MySpinner::new(0)), Default::default())
        .expect("spinner is mounted");

    app.active(&Id::Spinner).expect("spinner gets focus");

    let mut model = Model {
        app,
        quit: false,
        redraw: false,
        spinner_id: 0,
        terminal: TerminalBridge::new_crossterm().expect("bridge to be built"),
    };

    model
        .terminal
        .enter_alternate_screen()
        .expect("enter alt screen");
    model
        .terminal
        .enable_raw_mode()
        .expect("raw mode is enabled");

    while !model.quit {
        match model.app.tick(PollStrategy::Once) {
            Ok(messages) if !messages.is_empty() => {
                model.redraw = true;
                for msg in messages {
                    let mut msg = Some(msg);
                    while msg.is_some() {
                        msg = model.update(msg);
                    }
                }
            }
            _ => {}
        }

        if model.redraw {
            model.view();
            model.redraw = false;
        }
    }

    let _ = model.terminal.restore();
}
