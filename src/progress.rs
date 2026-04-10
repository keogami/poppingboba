use std::sync::{Arc, Mutex, MutexGuard};

use harmonica::{Spring, fps};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

use colorgrad::{CatmullRomGradient, Color as ColorGradColor, Gradient};

pub const DEFAULT_BLEND_START: u32 = 0x5A56E0; // Purple haze.
pub const DEFAULT_BLEND_END: u32 = 0xEE6FF8; // Neon pink.
pub const DEFAULT_FULL_COLOR: Color = Color::from_u32(0x7571F9); // Blueberry.
pub const DEFAULT_EMPTY_COLOR: Color = Color::from_u32(0x606060); // Slate gray.
pub const DEFAULT_FULL_CHAR_HALF_BLOCK: char = '▌';
pub const DEFAULT_EMPTY_CHAR_BLOCK: char = '░';

/// The character to use for filled spaces in the bar
pub enum Filled {
    /// Half block allows for higher resolution when doing color gradient
    Half(char),

    /// Full block only allows for lower resolution
    Full(char),
}

impl Filled {
    pub fn char(&self) -> char {
        match self {
            Filled::Half(c) => *c,
            Filled::Full(c) => *c,
        }
    }
}

pub struct Percentage {
    style: Style,
}

impl From<Style> for Percentage {
    fn from(value: Style) -> Self {
        Self { style: value }
    }
}

pub type ColorFn = dyn Fn(f64, f64) -> Color + Send + Sync + 'static;

/// A progress bar with gradient and animation capabilities
///
/// The width is clamped to the available area
struct ProgressInner {
    /// Max width, clamped to max available area
    width: Option<u16>,

    filled_char: Filled,
    filled_color: Color,

    empty_char: char,
    empty_color: Color,

    percentage: Option<Percentage>,

    /// when false, the entire width of the bar is used for color blending.
    /// when true, only the filled section's width is used for blending.
    scale_blend: bool,

    /// for dynamic coloring
    color_func: Option<Box<ColorFn>>,

    blend: Option<Vec<ColorGradColor>>,

    /// the actual progress
    progress: usize,
    max_progress: usize,
    /// the progress value shown when animating
    show_progress: f64,

    /// animation spring and velocity
    animation: Option<(Spring, f64)>,
}

#[derive(Clone)]
pub struct Progress {
    inner: Arc<Mutex<ProgressInner>>,
}

const fn u32_to_colorgrad_color(u: u32) -> ColorGradColor {
    let r = (u >> 16) as u8;
    let g = (u >> 8) as u8;
    let b = u as u8;

    ColorGradColor::from_rgba8(r, g, b, 255)
}

impl Progress {
    pub fn new(max: usize) -> Self {
        let inner = ProgressInner {
            width: None,
            filled_char: Filled::Half(DEFAULT_FULL_CHAR_HALF_BLOCK),
            filled_color: DEFAULT_FULL_COLOR,
            empty_char: DEFAULT_EMPTY_CHAR_BLOCK,
            empty_color: DEFAULT_EMPTY_COLOR,
            percentage: Some(Percentage {
                style: Default::default(),
            }),
            scale_blend: false,
            color_func: None,
            blend: Some(
                [DEFAULT_BLEND_START, DEFAULT_BLEND_END]
                    .into_iter()
                    .map(u32_to_colorgrad_color)
                    .collect(),
            ),
            progress: 0,
            show_progress: 0.,
            max_progress: max,
            animation: None,
        };

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    fn inner(&mut self) -> MutexGuard<'_, ProgressInner> {
        self.inner.lock().unwrap()
    }

    /// Set the colors for gradient. formatted as 0x00RRGGBB
    ///
    /// if there are no color is provided, default color is used.
    /// if there's only one color, the bar will be the solid color
    pub fn colors(mut self, colors: &[u32]) -> Self {
        match colors.len() {
            0 | 1 => {
                self.inner().filled_color = colors
                    .first()
                    .copied()
                    .map(Color::from_u32)
                    .unwrap_or(DEFAULT_FULL_COLOR);
                self.inner().blend = None;
                self.inner().color_func = None;
            }
            _ => {
                self.inner().blend =
                    Some(colors.iter().copied().map(u32_to_colorgrad_color).collect());
            }
        }

        self
    }

    /// Set a dynamic color function
    ///
    /// Disables the default color blending, handing the control over to the fn
    pub fn color_fn(mut self, f: impl Fn(f64, f64) -> Color + Send + Sync + 'static) -> Self {
        self.inner().color_func = Some(Box::new(f));
        self.inner().blend = None;
        self
    }

    /// Set the character for empty blocks
    pub fn empty(mut self, empty: char) -> Self {
        self.inner().empty_char = empty;
        self
    }

    /// Set the character for filled blocks
    pub fn filled(mut self, filled: Filled) -> Self {
        self.inner().filled_char = filled;
        self
    }

    /// Set how the percentage is styled
    pub fn percentage(mut self, p: impl Into<Percentage>) -> Self {
        self.inner().percentage = Some(p.into());
        self
    }

    /// Set the bar to not show any color
    pub fn no_percentage(mut self) -> Self {
        self.inner().percentage = None;
        self
    }

    /// Set the width for the progress bar, clamped to max area availabled
    pub fn width(mut self, w: u16) -> Self {
        self.inner().width = Some(w);
        self
    }

    /// Set whether to scale the blend/gradient to fit the width of only
    /// the filled portion of the progress bar. The default is false, which means the
    /// percentage must be 100% to see the full color blend/gradient.
    ///
    /// This is ignored when not using blending/multiple colors.
    pub fn scale_blend(mut self, enabled: bool) -> Self {
        self.inner().scale_blend = enabled;
        self
    }

    /// Tick the progress bar forward when using animations
    pub fn tick(&mut self) {
        let mut inner = self.inner();
        let Some((anim, vel)) = inner.animation else {
            return;
        };

        let (new_p, vel) = anim.update(inner.show_progress, vel, inner.progress as f64);

        inner.show_progress = new_p;

        if let Some(it) = inner.animation.as_mut() {
            it.1 = vel;
        }
    }

    /// Increments the progress by delta
    pub fn inc(&mut self, delta: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.progress = (inner.progress + delta).min(inner.max_progress);
        if inner.animation.is_none() {
            inner.show_progress = inner.progress as f64;
        }
    }

    /// Enables animation with the given spring
    pub fn animate(mut self, spring: Spring) -> Self {
        self.inner().animation = Some((spring, 0.));
        self
    }

    /// Enables animation with the given fps and default dampness
    pub fn animate_fps(mut self, f: u32) -> Self {
        let spring = Spring::new(fps(f), 18., 1.);
        self.inner().animation = Some((spring, 0.));
        self
    }

    /// Enables animation with 60 fps and default dampness
    pub fn animate_default(mut self) -> Self {
        let spring = Spring::new(fps(60), 18., 1.);
        self.inner().animation = Some((spring, 0.));
        self
    }
}

impl Widget for &Progress {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        self.inner.lock().unwrap().render(area, buf);
    }
}

impl Widget for &ProgressInner {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let max_width = self.width.unwrap_or(area.width).min(area.width);
        if max_width == 0 {
            return;
        }

        let percent = self.show_progress / self.max_progress as f64;

        let percent_str = self
            .percentage
            .as_ref()
            .map(|_| format!(" {:>05.1}", percent * 100.));
        let percent_width = percent_str.as_ref().map(|s| s.len()).unwrap_or_default() as u16;

        if max_width < percent_width {
            // can't print anything
            return;
        }

        // TODO: add size hints for allocations
        let mut spans = Vec::new();

        // bar view

        let tw = max_width.saturating_sub(percent_width);
        let fw = (tw as f64 * percent).round() as u16;
        let fw = fw.clamp(0, tw);

        let is_half_block = matches!(self.filled_char, Filled::Half(..));

        if let Some(get_color) = &self.color_func {
            // dynamic coloring
            let mut current;
            let half_block_perc = 0.5_f64 / tw as f64;

            for i in 0..fw {
                let mut style = Style::default();
                current = i as f64 / tw as f64;
                style = style.fg(get_color(percent, current));
                if is_half_block {
                    style = style.bg(get_color(percent, (current + half_block_perc).min(1.)));
                }

                spans.push(Span::from(self.filled_char.char().to_string()).style(style));
            }
        } else if let Some(blend) = self.blend.as_ref()
            && !blend.is_empty()
        {
            let multiplier = if is_half_block { 2 } else { 1 };

            let count = if self.scale_blend {
                fw * multiplier
            } else {
                tw * multiplier
            };

            // TODO: can potentially cache this and bubble up the error
            let gradient = colorgrad::GradientBuilder::new()
                .colors(blend)
                .mode(colorgrad::BlendMode::Oklab)
                .build::<CatmullRomGradient>()
                .unwrap();

            // TODO: can we avoid this allocation by using iter or gradient.at?
            let colors = gradient.colors(count as usize);
            let get_color = |i: u16| {
                colors
                    .get(i as usize)
                    .map(|c| {
                        let [r, g, b, _] = c.to_rgba8();
                        Color::Rgb(r, g, b)
                    })
                    .unwrap_or_default()
            };

            for i in 0..fw {
                let style = if !is_half_block {
                    Style::default().fg(get_color(i))
                } else {
                    Style::default()
                        .fg(get_color(i * 2))
                        .bg(get_color((i * 2) + 1))
                };

                spans.push(Span::from(self.filled_char.char().to_string()).style(style));
            }
        } else {
            let str: String = std::iter::repeat_n(self.filled_char.char(), fw as usize).collect();
            let style = Style::default().fg(self.filled_color).bg(self.filled_color);
            spans.push(Span::raw(str).style(style));
        }

        let str: String = std::iter::repeat_n(self.empty_char, (tw - fw) as usize).collect();
        let style = Style::default().fg(self.empty_color);
        spans.push(Span::raw(str).style(style));

        if let Some(perc) = percent_str {
            spans.push(
                Span::raw(perc).style(
                    self.percentage
                        .as_ref()
                        .map(|p| p.style)
                        .unwrap_or_default(),
                ),
            );
        }

        let line = Line::default().spans(spans);

        line.render(area, buf);
    }
}
