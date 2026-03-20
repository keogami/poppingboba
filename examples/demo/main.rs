use std::time::Duration;

use poppingboba::spinner::MockSpinner;
use tuirealm::{
    Application, Component, EventListenerCfg, MockComponent, NoUserEvent, PollStrategy, Update,
    command::Cmd,
    event::{Key, KeyEvent, KeyModifiers},
    ratatui::layout::{Constraint, Direction, Layout},
    terminal::{TerminalAdapter, TerminalBridge},
};

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
    component: MockSpinner,
}

impl Component<Msg, NoUserEvent> for MySpinner {
    fn on(&mut self, ev: tuirealm::Event<NoUserEvent>) -> Option<Msg> {
        let cmd = match ev {
            tuirealm::Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::AppClose),
            tuirealm::Event::Keyboard(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => Cmd::Tick,
            tuirealm::Event::Tick => Cmd::Tick,
            _ => Cmd::None,
        };

        match self.perform(cmd) {
            tuirealm::command::CmdResult::Changed(..) => Some(Msg::Tick),
            _ => None,
        }
    }
}

impl MySpinner {
    pub fn new() -> Self {
        Self {
            // component: MockSpinner::new(10., [".", "..", "...", "..."]),
            component: MockSpinner::new(10., &["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▰▰▱", "▰▱▱", "▱▱▱"]),
        }
    }
}

pub struct Model<T: TerminalAdapter> {
    // pub app: Application<Id, Msg, NoUserEvent>,
    pub quit: bool,
    pub redraw: bool,
    pub terminal: TerminalBridge<T>,
}

impl<T: TerminalAdapter> Model<T> {
    pub fn view(&mut self, app: &mut Application<Id, Msg, NoUserEvent>) {
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
                app.view(&Id::Spinner, frame, frame.area());
            })
            .expect("to render");
    }
}

impl<T: TerminalAdapter> Update<Msg> for Model<T> {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        let msg = msg?;

        self.redraw = true;
        match msg {
            Msg::AppClose => {
                self.quit = true;
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
            .tick_interval(Duration::from_millis(160)),
    );

    app.mount(Id::Spinner, Box::new(MySpinner::new()), Default::default())
        .expect("spinner is mounted");

    app.active(&Id::Spinner).expect("spinner gets focus");

    let mut model = Model {
        quit: false,
        redraw: false,
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
        match app.tick(PollStrategy::Once) {
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
            model.view(&mut app);
            model.redraw = false;
        }
    }

    let _ = model.terminal.restore();
}
