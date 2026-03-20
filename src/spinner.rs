use tuirealm::{
    MockComponent, State, StateValue,
    command::{Cmd, CmdResult},
    ratatui::text::Span,
};

pub struct Spinner {
    // will figure out fps later
    fps: f64,
    chars: &'static [&'static str],
}

impl Spinner {
    pub fn new(fps: f64, chars: &'static [&'static str]) -> Self {
        Self { fps, chars }
    }
}

pub struct MockSpinner {
    spinner: Spinner,
    frame: usize,
}

impl MockSpinner {
    pub fn new(fps: f64, chars: &'static [&'static str]) -> Self {
        Self {
            spinner: Spinner::new(fps, chars),
            frame: 0,
        }
    }
}

impl MockComponent for MockSpinner {
    fn view(&mut self, frame: &mut tuirealm::Frame, area: tuirealm::ratatui::prelude::Rect) {
        let s = self.spinner.chars[self.frame];
        frame.render_widget(Span::raw(s), area);
    }

    fn query(&self, _attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        None
    }

    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {
        // noop
    }

    fn state(&self) -> State {
        State::One(StateValue::Usize(self.frame))
    }

    fn perform(&mut self, cmd: Cmd) -> tuirealm::command::CmdResult {
        if cmd == Cmd::Tick {
            self.frame = (self.frame + 1) % self.spinner.chars.len();
            return CmdResult::Changed(self.state());
        }

        CmdResult::None
    }
}
