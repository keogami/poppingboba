use std::time::Duration;

use poppingboba::{
    help::{self, HelpWidget},
    key::{IntoBinding, KeyMap, KeyMapListener, ShareableKeyMap},
    spinner::{Spinner, SpinnerType},
};

use tuirealm::{
    Application, Attribute, Component, Event, EventListenerCfg, MockComponent, NoUserEvent,
    PollStrategy, Update,
    command::{Cmd, CmdResult},
    event::Key,
    ratatui::layout::{Constraint, Layout},
    terminal::{TerminalAdapter, TerminalBridge},
};

pub const GLOBAL_FPS: u32 = 60;

#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub enum Msg {
    AppClose,
    NextSpinner,
    PreviousSpinner,
    Tick,
    FullHelp,
    #[default]
    None,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Id {
    Spinner,
    GlobalListner,
    Shortcuts,
}

#[derive(MockComponent)]
struct MySpinner {
    component: Spinner,
}

impl Component<Msg, NoUserEvent> for MySpinner {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd = match ev {
            Event::Tick => Cmd::Tick,
            _ => Cmd::None,
        };

        match self.perform(cmd) {
            CmdResult::Changed(..) => Some(Msg::Tick),
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
    pub key_map: ShareableKeyMap<&'static str, Msg>,
    pub spinner_id: usize,
}

impl<T: TerminalAdapter> Model<T> {
    pub fn view(&mut self) {
        self.terminal
            .draw(|frame| {
                let [top_8_lines] = Layout::vertical([Constraint::Max(8)])
                    .flex(tuirealm::ratatui::layout::Flex::Start)
                    .areas(frame.area());
                let height = self
                    .app
                    .query(&Id::Shortcuts, Attribute::Height)
                    .unwrap()
                    .unwrap()
                    .unwrap_size();
                let [spinner, keyboard] =
                    Layout::vertical([Constraint::Length(1), Constraint::Length(height)])
                        .flex(tuirealm::ratatui::layout::Flex::SpaceBetween)
                        .areas(top_8_lines);
                self.app.view(&Id::Spinner, frame, spinner);
                self.app.view(&Id::Shortcuts, frame, keyboard);
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
            Msg::FullHelp => {
                let prev = self
                    .app
                    .query(&Id::Shortcuts, Attribute::Custom(help::SHOW_FULL))
                    .ok()
                    .flatten()
                    .and_then(|value| value.as_flag())
                    .unwrap_or(true);

                self.app
                    .attr(
                        &Id::Shortcuts,
                        Attribute::Custom(help::SHOW_FULL),
                        tuirealm::AttrValue::Flag(!prev),
                    )
                    .ok();

                None
            }
            Msg::Tick | Msg::None => None,
        }
    }
}

#[derive(MockComponent)]
struct Shortcuts {
    component: HelpWidget<&'static str, KeyMap<&'static str, Msg>>,
}

impl Component<Msg, NoUserEvent> for Shortcuts {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Msg> {
        None
    }
}

fn main() {
    let mut app: Application<Id, Msg, NoUserEvent> = Application::init(
        EventListenerCfg::default()
            .crossterm_input_listener(Duration::from_millis(20), 2)
            .tick_interval(Duration::from_secs(1) / GLOBAL_FPS),
    );

    let key_map = KeyMap::from([
        (
            "quit",
            [Key::Char('q'), Key::Esc]
                .into_binding()
                .help(("q/esc", "quit"))
                .message(Msg::AppClose),
        ),
        (
            "prev",
            [Key::Char('k'), Key::Up]
                .into_binding()
                .help(("k/up", "previous spinner"))
                .message(Msg::PreviousSpinner),
        ),
        (
            "next",
            [Key::Char('j'), Key::Down]
                .into_binding()
                .help(("j/down", "next spinner"))
                .message(Msg::NextSpinner),
        ),
        (
            "full",
            [Key::Char('?'), Key::Down]
                .into_binding()
                .help(("?", "toggle help"))
                .message(Msg::FullHelp),
        ),
    ]);
    let key_map = key_map
        .short_help(&["full", "prev", "next"])
        .full_help(3, &["quit", "next", "prev", "full"]);
    let key_map = key_map.shareable();

    app.mount(Id::Spinner, Box::new(MySpinner::new(0)), Default::default())
        .expect("spinner is mounted");

    let (listener, subs) = KeyMapListener::new(key_map.clone());
    app.mount(Id::GlobalListner, Box::new(listener), subs)
        .expect("listener to be mounted");

    app.mount(
        Id::Shortcuts,
        Box::new(Shortcuts {
            component: HelpWidget::from(key_map.clone()),
        }),
        Default::default(),
    )
    .expect("Mount shortcut component");

    app.active(&Id::Spinner).expect("spinner gets focus");

    let mut model = Model {
        app,
        quit: false,
        redraw: false,
        spinner_id: 0,
        key_map,
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
