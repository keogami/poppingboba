use tuirealm::{
    MockComponent, State, StateValue,
    command::{Cmd, CmdResult},
    props::Style,
    ratatui::{style::Styled, text::Span, widgets::Widget},
};

#[derive(Clone, Copy)]
pub struct SpinnerType {
    fps: u32,
    chars: &'static [&'static str],
}

impl SpinnerType {
    pub const fn new(fps: u32, chars: &'static [&'static str]) -> Self {
        Self { fps, chars }
    }

    pub const fn ellipsis() -> Self {
        Self::new(3, &["", ".", "..", "..."])
    }

    pub const fn line() -> Self {
        Self::new(10, &["|", "/", "-", "\\"])
    }

    pub const fn dot() -> Self {
        Self::new(10, &["⣾ ", "⣽ ", "⣻ ", "⢿ ", "⡿ ", "⣟ ", "⣯ ", "⣷ "])
    }

    pub const fn mini_dot() -> Self {
        Self::new(12, &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
    }

    pub const fn jump() -> Self {
        Self::new(10, &["⢄", "⢂", "⢁", "⡁", "⡈", "⡐", "⡠"])
    }

    pub const fn pulse() -> Self {
        Self::new(8, &["█", "▓", "▒", "░"])
    }

    pub const fn globe() -> Self {
        Self::new(4, &["🌍", "🌎", "🌏"])
    }

    pub const fn moon() -> Self {
        Self::new(8, &["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"])
    }

    pub const fn monkey() -> Self {
        Self::new(3, &["🙈", "🙉", "🙊"])
    }

    pub const fn meter() -> Self {
        Self::new(3, &["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▰▰▱", "▰▱▱", "▱▱▱"])
    }

    pub const fn hamburger() -> Self {
        Self::new(3, &["☱", "☲", "☴", "☲"])
    }
}

pub struct Spinner {
    spinner: SpinnerType,
    frame: usize,
    global_fps: u32,
    style: Style,
}

impl Styled for Spinner {
    type Item = Self;

    fn style(&self) -> Style {
        self.style
    }

    fn set_style<S: Into<Style>>(mut self, style: S) -> Self::Item {
        self.style = style.into();
        self
    }
}

impl Spinner {
    pub fn new(spinner: SpinnerType, fps: u32) -> Self {
        Self {
            spinner,
            frame: 0,
            global_fps: fps,
            style: Style::default(),
        }
    }

    pub fn tick(&mut self) {
        self.frame = (self.frame + 1) % (self.global_fps as usize * self.spinner.chars.len());
    }
}

impl Widget for &Spinner {
    fn render(
        self,
        area: tuirealm::ratatui::layout::Rect,
        buf: &mut tuirealm::ratatui::buffer::Buffer,
    ) where
        Self: Sized,
    {
        // downsizing global fps to local fps. say 60fps global going down to
        // 10fps for dot spinner which has 8frames.
        //
        // note: used f32 for guaranteed correct results, raise pr with usize if
        // you can prove it wont cause animation to be jumpy
        let current_frame = (self.spinner.fps as f32 * self.frame as f32) / self.global_fps as f32;
        let current_frame = current_frame % self.spinner.chars.len() as f32;
        let current_frame = current_frame.floor();

        let span = Span::styled(self.spinner.chars[current_frame as usize], self.style);

        span.render(area, buf);
    }
}

impl MockComponent for Spinner {
    fn view(&mut self, frame: &mut tuirealm::Frame, area: tuirealm::ratatui::prelude::Rect) {
        self.render(area, frame.buffer_mut());
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
        // FIXME: we are reporting a change at every frame even if the actual
        // spinner has a much tinier fps, say 5 or 10
        //
        // It is possible to reduce the change result but im not sure whether
        // keeping extra information is worth the reduced CmdResult::Changed
        // returned
        if cmd == Cmd::Tick {
            self.tick();
            return CmdResult::Changed(self.state());
        }

        CmdResult::None
    }
}
