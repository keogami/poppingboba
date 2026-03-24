use std::time::Duration;

use poppingboba::{
    help::{self, HelpWidget},
    key::{IntoBinding, KeyMap, KeyMapListener},
    progress::{Filled, Progress},
};

use tuirealm::{
    Application, Attribute, Component, Event, EventListenerCfg, MockComponent, NoUserEvent,
    PollStrategy, Update,
    command::Cmd,
    event::Key,
    props::{Color, Style},
    ratatui::layout::{Constraint, Layout},
    terminal::{TerminalAdapter, TerminalBridge},
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

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Id {
    DefaultBar,
    SolidBar,
    BlendBar,
    DynamicBar,
    NoPercentBar,
    FullBlockBar,
    AnimatedBar,
    GlobalListener,
    Shortcuts,
}

#[derive(MockComponent)]
struct ProgressBar {
    component: Progress,
}

impl Component<Msg, NoUserEvent> for ProgressBar {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Tick => {
                self.perform(Cmd::Tick);
                Some(Msg::Tick)
            }
            _ => None,
        }
    }
}

struct Bars {
    default: Progress,
    solid: Progress,
    blend: Progress,
    dynamic: Progress,
    no_percent: Progress,
    full_block: Progress,
    animated: Progress,
}

pub struct Model<T: TerminalAdapter> {
    pub app: Application<Id, Msg, NoUserEvent>,
    pub quit: bool,
    pub redraw: bool,
    pub terminal: TerminalBridge<T>,
    pub tick_counter: usize,
    bars: Bars,
    last_progress: usize,
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

impl<T: TerminalAdapter> Model<T> {
    pub fn view(&mut self) {
        self.terminal
            .draw(|frame| {
                let help_height = self
                    .app
                    .query(&Id::Shortcuts, Attribute::Height)
                    .unwrap()
                    .unwrap()
                    .unwrap_size();

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
                    .flex(tuirealm::ratatui::layout::Flex::Start)
                    .split(frame.area());

                let bar_ids = [
                    Id::DefaultBar,
                    Id::SolidBar,
                    Id::BlendBar,
                    Id::DynamicBar,
                    Id::NoPercentBar,
                    Id::FullBlockBar,
                    Id::AnimatedBar,
                ];

                for (i, id) in bar_ids.iter().enumerate() {
                    // Each bar is at index i*2 (bar 0 at 0, bar 1 at 2, bar 2 at 4, ...)
                    // because spacer rows sit between them at odd indices
                    let area_idx = i * 2;
                    let label_width = LABELS[i].len() as u16;
                    let [label_area, bar_area] =
                        Layout::horizontal([Constraint::Length(label_width), Constraint::Min(0)])
                            .areas(areas[area_idx]);

                    let label_style = Style::default().fg(Color::from_u32(0xBB6BD9));
                    tuirealm::ratatui::widgets::Paragraph::new(LABELS[i])
                        .style(label_style)
                        .render(label_area, frame.buffer_mut());

                    self.app.view(id, frame, bar_area);
                }

                self.app.view(&Id::Shortcuts, frame, *areas.last().unwrap());
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
            Msg::Tick => {
                self.tick_counter += 1;

                for &(tick, target) in &CHECKPOINTS {
                    if self.tick_counter == tick {
                        let delta = target - self.last_progress;
                        self.last_progress = target;

                        self.bars.default.inc(delta);
                        self.bars.solid.inc(delta);
                        self.bars.blend.inc(delta);
                        self.bars.dynamic.inc(delta);
                        self.bars.no_percent.inc(delta);
                        self.bars.full_block.inc(delta);
                        self.bars.animated.inc(delta);
                    }
                }

                self.bars.animated.tick();

                None
            }
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

fn purple_color_fn(percent: f64, position: f64) -> Color {
    // Interpolate from deep purple to fuchsia based on a mix of progress and position
    let t = (percent * 0.5 + position * 0.5).clamp(0.0, 1.0);

    // Deep purple (0x8E44AD) -> Fuchsia (0xD946EF)
    let r = (0x8E as f64 + (0xD9 as f64 - 0x8E as f64) * t) as u8;
    let g = (0x44 as f64 + (0x46 as f64 - 0x44 as f64) * t) as u8;
    let b = (0xAD as f64 + (0xEF as f64 - 0xAD as f64) * t) as u8;

    Color::Rgb(r, g, b)
}

use tuirealm::ratatui::widgets::Widget;

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
            "full",
            [Key::Char('?')]
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
    let default_bar = Progress::new(MAX_PROGRESS);
    let solid_bar = Progress::new(MAX_PROGRESS).colors(&[0x9B59B6]);
    let blend_bar = Progress::new(MAX_PROGRESS)
        .colors(&[0x8E44AD, 0xBB6BD9, 0xF0ABFC])
        .scale_blend(true);
    let dynamic_bar = Progress::new(MAX_PROGRESS)
        .color_fn(purple_color_fn)
        .no_percentage();
    let no_percent_bar = Progress::new(MAX_PROGRESS)
        .colors(&[0x8E44AD, 0xD946EF])
        .no_percentage();
    let full_block_bar = Progress::new(MAX_PROGRESS)
        .colors(&[0x8E44AD, 0xBB6BD9, 0xF0ABFC])
        .filled(Filled::Full('\u{2588}'));
    let animated_bar = Progress::new(MAX_PROGRESS).animate_fps(GLOBAL_FPS);

    let bars = Bars {
        default: default_bar.clone(),
        solid: solid_bar.clone(),
        blend: blend_bar.clone(),
        dynamic: dynamic_bar.clone(),
        no_percent: no_percent_bar.clone(),
        full_block: full_block_bar.clone(),
        animated: animated_bar.clone(),
    };

    app.mount(
        Id::DefaultBar,
        Box::new(ProgressBar {
            component: default_bar,
        }),
        Default::default(),
    )
    .expect("mount default bar");

    app.mount(
        Id::SolidBar,
        Box::new(ProgressBar {
            component: solid_bar,
        }),
        Default::default(),
    )
    .expect("mount solid bar");

    app.mount(
        Id::BlendBar,
        Box::new(ProgressBar {
            component: blend_bar,
        }),
        Default::default(),
    )
    .expect("mount blend bar");

    app.mount(
        Id::DynamicBar,
        Box::new(ProgressBar {
            component: dynamic_bar,
        }),
        Default::default(),
    )
    .expect("mount dynamic bar");

    app.mount(
        Id::NoPercentBar,
        Box::new(ProgressBar {
            component: no_percent_bar,
        }),
        Default::default(),
    )
    .expect("mount no percent bar");

    app.mount(
        Id::FullBlockBar,
        Box::new(ProgressBar {
            component: full_block_bar,
        }),
        Default::default(),
    )
    .expect("mount full block bar");

    app.mount(
        Id::AnimatedBar,
        Box::new(ProgressBar {
            component: animated_bar,
        }),
        Default::default(),
    )
    .expect("mount animated bar");

    let (listener, subs) = KeyMapListener::new(key_map.clone());
    app.mount(Id::GlobalListener, Box::new(listener), subs)
        .expect("mount listener");

    app.mount(
        Id::Shortcuts,
        Box::new(Shortcuts {
            component: HelpWidget::from(key_map.clone()),
        }),
        Default::default(),
    )
    .expect("mount shortcuts");

    app.active(&Id::DefaultBar).expect("default bar gets focus");

    let mut model = Model {
        app,
        quit: false,
        redraw: false,
        terminal: TerminalBridge::init_crossterm().expect("terminal init"),
        tick_counter: 0,
        bars,
        last_progress: 0,
    };

    while !model.quit {
        match model.app.tick(PollStrategy::Once) {
            Ok(messages) if !messages.is_empty() => {
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
