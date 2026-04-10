use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Flex, Layout},
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

use poppingboba::{
    help::{HelpState, HelpWidget},
    key::{IntoBinding, KeyMap},
    progress::{Filled, Progress},
};

pub const GLOBAL_FPS: u32 = 60;
pub const MAX_PROGRESS: usize = 390;

/// (tick, target_progress) — at the given tick, set cumulative progress to target.
/// Shared across all bars.
const CHECKPOINTS: [(usize, usize); 3] = [(100, 100), (200, 300), (MAX_PROGRESS, 1000)];

#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub enum Msg {
    AppClose,
    #[default]
    Tick,
    FullHelp,
}

const BAR_COUNT: u16 = 7;

const LABELS: [&str; BAR_COUNT as usize] = [
    "   Default ",
    "     Solid ",
    "     Blend ",
    "   Dynamic ",
    "  No Prcnt ",
    "Full Block ",
    "  Animated ",
];

fn purple_color_fn(percent: f64, position: f64) -> Color {
    // Interpolate from deep purple to fuchsia based on a mix of progress and position
    let t = (percent * 0.5 + position * 0.5).clamp(0.0, 1.0);

    // Deep purple (0x8E44AD) -> Fuchsia (0xD946EF)
    let r = (0x8E as f64 + (0xD9 as f64 - 0x8E as f64) * t) as u8;
    let g = (0x44 as f64 + (0x46 as f64 - 0x44 as f64) * t) as u8;
    let b = (0xAD as f64 + (0xEF as f64 - 0xAD as f64) * t) as u8;

    Color::Rgb(r, g, b)
}

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
            "full",
            [KeyCode::Char('?')]
                .into_binding()
                .help(("?", "toggle help"))
                .message(Msg::FullHelp),
        ),
    ]);
    let key_map = key_map
        .short_help(&["full", "quit"])
        .full_help(2, &["quit", "full"]);
    let key_map = key_map.shareable();

    // Create progress bars — clones share state via Arc<Mutex>
    let mut default_bar = Progress::new(MAX_PROGRESS);
    let mut solid_bar = Progress::new(MAX_PROGRESS).colors(&[0x9B59B6]);
    let mut blend_bar = Progress::new(MAX_PROGRESS)
        .colors(&[0x8E44AD, 0xBB6BD9, 0xF0ABFC])
        .scale_blend(true);
    let mut dynamic_bar = Progress::new(MAX_PROGRESS)
        .color_fn(purple_color_fn)
        .no_percentage();
    let mut no_percent_bar = Progress::new(MAX_PROGRESS)
        .colors(&[0x8E44AD, 0xD946EF])
        .no_percentage();
    let mut full_block_bar = Progress::new(MAX_PROGRESS)
        .colors(&[0x8E44AD, 0xBB6BD9, 0xF0ABFC])
        .filled(Filled::Full('\u{2588}'));
    let mut animated_bar = Progress::new(MAX_PROGRESS).animate_fps(GLOBAL_FPS);

    let mut help = HelpWidget::from(key_map.clone());
    let mut tick_counter: usize = 0;
    let mut last_progress: usize = 0;

    let mut terminal = ratatui::init();
    let tick_rate = Duration::from_secs(1) / GLOBAL_FPS;

    loop {
        terminal
            .draw(|frame| {
                let help_height = help.height();

                let mut bar_constraints: Vec<Constraint> = Vec::new();
                for i in 0..BAR_COUNT as usize {
                    if i > 0 {
                        bar_constraints.push(Constraint::Length(1)); // spacer between bars
                    }
                    bar_constraints.push(Constraint::Length(1)); // bar
                }
                bar_constraints.push(Constraint::Length(2)); // spacer before help
                bar_constraints.push(Constraint::Length(help_height));

                let areas = Layout::vertical(bar_constraints)
                    .flex(Flex::Start)
                    .split(frame.area());

                let bars: [&Progress; BAR_COUNT as usize] = [
                    &default_bar,
                    &solid_bar,
                    &blend_bar,
                    &dynamic_bar,
                    &no_percent_bar,
                    &full_block_bar,
                    &animated_bar,
                ];

                for (i, bar) in bars.iter().enumerate() {
                    let area_idx = i * 2;
                    let label_width = LABELS[i].len() as u16;
                    let [label_area, bar_area] =
                        Layout::horizontal([Constraint::Length(label_width), Constraint::Min(0)])
                            .areas(areas[area_idx]);

                    let label_style = Style::default().fg(Color::from_u32(0xBB6BD9));
                    Paragraph::new(LABELS[i])
                        .style(label_style)
                        .render(label_area, frame.buffer_mut());

                    (*bar).render(bar_area, frame.buffer_mut());
                }

                (&help).render(*areas.last().unwrap(), frame.buffer_mut());
            })
            .expect("to render");

        if event::poll(tick_rate).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if let Some(msg) = key_map.borrow().match_key_event(key) {
                    match msg {
                        Msg::AppClose => break,
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

        // tick progress
        tick_counter += 1;
        for &(tick, target) in &CHECKPOINTS {
            if tick_counter == tick {
                let delta = target - last_progress;
                last_progress = target;

                default_bar.inc(delta);
                solid_bar.inc(delta);
                blend_bar.inc(delta);
                dynamic_bar.inc(delta);
                no_percent_bar.inc(delta);
                full_block_bar.inc(delta);
                animated_bar.inc(delta);
            }
        }

        animated_bar.tick();
    }

    ratatui::restore();
}
