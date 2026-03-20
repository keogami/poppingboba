use std::{cell::RefCell, rc::Rc, time::Duration};

use poppingboba::{
    key::{Binding, Help, KeyMap},
    spinner::{Spinner, SpinnerType},
};
use tui_realm_stdlib::Phantom;
use tuirealm::{
    Application, Component, Event, EventListenerCfg, MockComponent, NoUserEvent, PollStrategy, Sub,
    SubClause, SubEventClause, Update,
    command::{Cmd, Direction},
    event::Key,
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
    GlobalListner,
}

#[derive(MockComponent)]
struct MySpinner {
    component: Spinner,
}

#[derive(MockComponent)]
struct GlobalListner {
    component: Phantom,
    key_map: Rc<RefCell<KeyMap<&'static str>>>,
}

impl GlobalListner {
    fn new(key_map: Rc<RefCell<KeyMap<&'static str>>>) -> Self {
        Self {
            key_map,
            component: Default::default(),
        }
    }
}

impl Component<Msg, NoUserEvent> for GlobalListner {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd = match ev {
            Event::Keyboard(ev) => self.key_map.borrow().match_key_event(ev),
            _ => Cmd::None,
        };

        match cmd {
            Cmd::Move(direction) => match direction {
                Direction::Down => Some(Msg::NextSpinner),
                Direction::Up => Some(Msg::PreviousSpinner),
                _ => None,
            },
            Cmd::Custom("quit") => Some(Msg::AppClose),
            _ => None,
        }
    }
}

impl Component<Msg, NoUserEvent> for MySpinner {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd = match ev {
            Event::Tick => Cmd::Tick,
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

    let key_map = KeyMap::from([
        (
            "quit",
            Binding::new([Key::Char('q'), Key::Esc], Help::new("q/esc", "quit"))
                .command(Cmd::Custom("quit")),
        ),
        (
            "prev",
            Binding::new([Key::Char('k'), Key::Up], Help::new("k/up", "prev"))
                .command(Cmd::Move(Direction::Up)),
        ),
        (
            "next",
            Binding::new([Key::Char('j'), Key::Down], Help::new("j/down", "next"))
                .command(Cmd::Move(Direction::Down)),
        ),
    ]);
    let key_map = Rc::new(RefCell::new(key_map));

    app.mount(Id::Spinner, Box::new(MySpinner::new(0)), Default::default())
        .expect("spinner is mounted");

    app.mount(
        Id::GlobalListner,
        Box::new(GlobalListner::new(key_map.clone())),
        vec![Sub::new(SubEventClause::Any, SubClause::Always)],
    )
    .expect("listener to be mounted");

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
