use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Flex, Layout},
    widgets::Widget,
};

use poppingboba::{
    help::{HelpState, HelpWidget},
    key::{IntoBinding, KeyMap},
    spinner::{Spinner, SpinnerType},
};

pub const GLOBAL_FPS: u32 = 60;

#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub enum Msg {
    AppClose,
    NextSpinner,
    PreviousSpinner,
    #[default]
    Tick,
    FullHelp,
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

fn main() {
    let key_map = KeyMap::from([
        (
            "quit",
            [KeyCode::Char('q'), KeyCode::Esc]
                .into_binding()
                .help(("q/esc", "quit"))
                .message(Msg::AppClose),
        ),
        (
            "prev",
            [KeyCode::Char('k'), KeyCode::Up]
                .into_binding()
                .help(("k/up", "previous spinner"))
                .message(Msg::PreviousSpinner),
        ),
        (
            "next",
            [KeyCode::Char('j'), KeyCode::Down]
                .into_binding()
                .help(("j/down", "next spinner"))
                .message(Msg::NextSpinner),
        ),
        (
            "full",
            [KeyCode::Char('?')]
                .into_binding()
                .help(("?", "toggle help"))
                .message(Msg::FullHelp),
        ),
    ]);
    let key_map = key_map
        .short_help(&["full", "prev", "next"])
        .full_help(3, &["quit", "next", "prev", "full"]);
    let key_map = key_map.shareable();

    let mut spinner = Spinner::new(SPINNERS[0], GLOBAL_FPS);
    let mut spinner_id: usize = 0;
    let mut help = HelpWidget::from(key_map.clone());

    let mut terminal = ratatui::init();
    let tick_rate = Duration::from_secs(1) / GLOBAL_FPS;

    loop {
        terminal
            .draw(|frame| {
                let [top_8_lines] = Layout::vertical([Constraint::Max(8)])
                    .flex(Flex::Start)
                    .areas(frame.area());
                let height = help.height();
                let [spinner_area, keyboard] =
                    Layout::vertical([Constraint::Length(1), Constraint::Length(height)])
                        .flex(Flex::SpaceBetween)
                        .areas(top_8_lines);
                (&spinner).render(spinner_area, frame.buffer_mut());
                (&help).render(keyboard, frame.buffer_mut());
            })
            .expect("to render");

        if event::poll(tick_rate).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if let Some(msg) = key_map.borrow().match_key_event(key) {
                    match msg {
                        Msg::AppClose => break,
                        Msg::NextSpinner => {
                            spinner_id = (spinner_id + 1) % SPINNERS.len();
                            spinner = Spinner::new(SPINNERS[spinner_id], GLOBAL_FPS);
                        }
                        Msg::PreviousSpinner => {
                            spinner_id = if spinner_id == 0 {
                                SPINNERS.len() - 1
                            } else {
                                spinner_id - 1
                            };
                            spinner = Spinner::new(SPINNERS[spinner_id], GLOBAL_FPS);
                        }
                        Msg::FullHelp => {
                            let new_state = match help.state() {
                                HelpState::Full => HelpState::Short,
                                HelpState::Short => HelpState::Full,
                            };
                            help.set_state(new_state);
                        }
                        Msg::Tick => {}
                    }
                }
            }
        }

        spinner.tick();
    }

    ratatui::restore();
}
