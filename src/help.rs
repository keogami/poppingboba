use std::{borrow::Cow, cell::RefCell, collections::BTreeMap, marker::PhantomData, rc::Rc};

use tuirealm::{
    MockComponent,
    props::{Color, Style},
    ratatui::{
        layout::{Constraint, Layout},
        text::{Line, Span},
        widgets::Widget,
    },
};

pub const SHOW_FULL: &str = "help_state";

const ELLIPSIS: &str = "…";
const SHORT_SEP: &str = " • ";
const FULL_SEP: &str = "    ";

/// Help information for bindings
pub struct Help {
    /// Printable key string for the binding
    pub key: Cow<'static, str>,
    /// A short description of the binding
    pub desc: Cow<'static, str>,
}

impl Help {
    pub fn new(key: impl Into<Cow<'static, str>>, desc: impl Into<Cow<'static, str>>) -> Self {
        Self {
            key: key.into(),
            desc: desc.into(),
        }
    }
}

impl<Key: Into<Cow<'static, str>>, Desc: Into<Cow<'static, str>>> From<(Key, Desc)> for Help {
    fn from((key, desc): (Key, Desc)) -> Self {
        Self {
            key: key.into(),
            desc: desc.into(),
        }
    }
}

pub struct HelpTable<T: Clone + 'static> {
    pub map: BTreeMap<T, Help>,
    pub short: Cow<'static, [T]>,
    pub full: (Cow<'static, [T]>, usize),
}

impl<T: Clone + 'static> HelpTable<T> {
    pub fn new(
        map: impl Into<BTreeMap<T, Help>>,
        short: impl Into<Cow<'static, [T]>>,
        full: (impl Into<Cow<'static, [T]>>, usize),
    ) -> Self {
        Self {
            map: map.into(),
            short: short.into(),
            full: (full.0.into(), full.1),
        }
    }
}

/// Help information getter
///
/// Allows the implementor to decide how to store the help information
pub trait HelpInfo<KeyId> {
    /// Gives all the keys that should be shown in the short view
    fn short_help(&self) -> &[KeyId];

    /// Gives all the keys that should be show in the full view, and the number of columns
    fn full_help(&self) -> (&[KeyId], usize);

    /// Getter for the actual help information
    ///
    /// It is the implementors' responsibility to make sure all the keys returned have a valid help information
    fn help(&self, key_id: &KeyId) -> &Help;
}

impl<KeyId: Clone + Ord> HelpInfo<KeyId> for HelpTable<KeyId> {
    fn short_help(&self) -> &[KeyId] {
        self.short.as_ref()
    }

    fn full_help(&self) -> (&[KeyId], usize) {
        (self.full.0.as_ref(), self.full.1)
    }

    fn help(&self, key_id: &KeyId) -> &Help {
        self.map
            .get(key_id)
            .expect("The user has provided valid keys")
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum HelpState {
    Full,
    Short,
}

pub struct HelpWidget<KeyId, Info> {
    help_info: Rc<RefCell<Info>>,
    state: HelpState,
    short_separator: (Cow<'static, str>, Style),
    short_key: Style,
    short_desc: Style,
    full_separator: (Cow<'static, str>, Style),
    full_key: Style,
    full_desc: Style,
    ellipsis: (Cow<'static, str>, Style),
    _marker: PhantomData<KeyId>,
}

impl<KeyId, Info: HelpInfo<KeyId>> From<Rc<RefCell<Info>>> for HelpWidget<KeyId, Info> {
    fn from(value: Rc<RefCell<Info>>) -> Self {
        // TODO: make this configurable
        let is_dark = true;

        let key_style =
            Style::new().fg(Color::from_u32(if !is_dark { 0x909090 } else { 0x626262 }));
        let desc_style =
            Style::new().fg(Color::from_u32(if !is_dark { 0xB2B2B2 } else { 0x4A4A4A }));
        let sep_style =
            Style::new().fg(Color::from_u32(if !is_dark { 0xDADADA } else { 0x3C3C3C }));

        Self {
            help_info: value.clone(),
            state: HelpState::Short,
            short_separator: (SHORT_SEP.into(), sep_style),
            short_key: key_style,
            short_desc: desc_style,
            full_separator: (FULL_SEP.into(), sep_style),
            full_key: key_style,
            full_desc: desc_style,
            ellipsis: (ELLIPSIS.into(), sep_style),
            _marker: PhantomData,
        }
    }
}

impl<KeyId, T> Widget for &HelpWidget<KeyId, T>
where
    KeyId: Clone,
    T: HelpInfo<KeyId>,
{
    fn render(
        self,
        area: tuirealm::ratatui::prelude::Rect,
        buf: &mut tuirealm::ratatui::prelude::Buffer,
    ) where
        Self: Sized,
    {
        match self.state {
            HelpState::Full => {
                let separator = &self.full_separator.0;
                let sep_width = separator.len() as u16;
                let ellipsis = format!(" {}", self.ellipsis.0);
                let ellipsis_width = ellipsis.len() as u16;
                let help_info = self.help_info.borrow();
                let (full_keys, rows) = help_info.full_help();

                if rows == 0 || full_keys.is_empty() {
                    return;
                }

                let columns: Vec<&[KeyId]> = full_keys.chunks(rows).collect();

                // measure which columns fit
                let mut col_widths: Vec<u16> = Vec::new();
                let mut col_key_widths: Vec<usize> = Vec::new();
                let mut total_width: u16 = 0;
                let mut needs_ellipsis = false;

                for (i, col) in columns.iter().enumerate() {
                    let max_key_width = col
                        .iter()
                        .map(|k| help_info.help(k).key.len())
                        .max()
                        .unwrap_or(0);
                    let max_desc_width = col
                        .iter()
                        .map(|k| help_info.help(k).desc.len())
                        .max()
                        .unwrap_or(0);
                    let col_width = max_key_width as u16 + 1 + max_desc_width as u16;

                    let with_sep = if i == 0 { 0 } else { sep_width };
                    let needed = with_sep + col_width;

                    if total_width + needed <= area.width {
                        total_width += needed;
                        col_widths.push(needed);
                        col_key_widths.push(max_key_width);
                    } else {
                        needs_ellipsis = true;
                        // remove columns until ellipsis fits
                        while !col_widths.is_empty() && total_width + ellipsis_width > area.width {
                            total_width -= col_widths.pop().unwrap();
                            col_key_widths.pop();
                        }
                        break;
                    }
                }

                if col_widths.is_empty() {
                    return;
                }

                // build horizontal constraints
                let mut constraints: Vec<Constraint> =
                    col_widths.iter().map(|&w| Constraint::Length(w)).collect();
                if needs_ellipsis {
                    constraints.push(Constraint::Length(ellipsis_width));
                }

                let col_areas = Layout::horizontal(constraints).split(area);

                // render each column
                let row_constraints: Vec<Constraint> =
                    (0..rows).map(|_| Constraint::Length(1)).collect();

                for (i, col) in columns[..col_widths.len()].iter().enumerate() {
                    let row_areas = Layout::vertical(row_constraints.clone()).split(col_areas[i]);

                    for (j, key_id) in col.iter().enumerate() {
                        let h = help_info.help(key_id);
                        let mut spans = Vec::with_capacity(4);

                        if i != 0 {
                            spans.push(
                                Span::raw(separator.to_string()).style(self.full_separator.1),
                            );
                        }

                        let padded_key =
                            format!("{:<width$}", h.key, width = col_key_widths[i]);
                        spans.push(Span::raw(padded_key).style(self.full_key));
                        spans.push(Span::raw(" "));
                        spans.push(Span::raw(h.desc.clone()).style(self.full_desc));

                        Line::default().spans(spans).render(row_areas[j], buf);
                    }
                }

                // render ellipsis
                if needs_ellipsis {
                    let ellipsis_area = col_areas[col_widths.len()];
                    Line::raw(ellipsis)
                        .style(self.ellipsis.1)
                        .render(ellipsis_area, buf);
                }
            }
            HelpState::Short => {
                let separator = &self.short_separator.0;
                let ellipsis = format!(" {}", self.ellipsis.0);
                let help_info = self.help_info.borrow();
                let short_keys = help_info.short_help();
                let item_width = |key: &KeyId| {
                    let h = help_info.help(key);
                    h.key.len() as u16 + 1u16 + h.desc.len() as u16
                };

                let mut count: usize = 0;
                let mut total: u16 = 0;

                // measure how many items we can add without overflowing
                for (i, item) in short_keys.iter().enumerate() {
                    let sep: u16 = if i == 0 { 0 } else { separator.len() as u16 };
                    let added_width: u16 = sep + item_width(item);
                    if added_width + total <= area.width {
                        count += 1;
                        total += added_width;
                    } else {
                        break;
                    }
                }

                // needs ellipsis if all the items weren't added
                let needs_ellipsis = count != short_keys.len();
                if needs_ellipsis {
                    // remove items until there's space for the separator
                    while count > 0 && total + ellipsis.len() as u16 > area.width {
                        count -= 1;
                        let sep = if count == 0 {
                            0
                        } else {
                            separator.len() as u16
                        };
                        let removed_width = sep + item_width(&short_keys[count]);
                        total -= removed_width;
                    }
                }

                // finally build the line
                let mut spans = Vec::with_capacity((count * 3).max(1)); // rough estimation

                for (i, item) in short_keys[..count].iter().enumerate() {
                    if i != 0 {
                        spans.push(Span::raw(separator.to_string()).style(self.short_separator.1));
                    }

                    spans.push(Span::raw(help_info.help(item).key.clone()).style(self.short_key));
                    spans.push(Span::raw(" "));
                    spans.push(Span::raw(help_info.help(item).desc.clone()).style(self.short_desc));
                }
                if needs_ellipsis {
                    spans.push(Span::raw(ellipsis).style(self.ellipsis.1));
                }

                let line = Line::default().spans(spans);

                line.render(area, buf);
            }
        }
    }
}

impl<KeyId: Clone, T: HelpInfo<KeyId>> MockComponent for HelpWidget<KeyId, T> {
    fn view(&mut self, frame: &mut tuirealm::Frame, area: tuirealm::ratatui::prelude::Rect) {
        self.render(area, frame.buffer_mut());
    }

    fn query(&self, attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        match attr {
            tuirealm::Attribute::Height => Some(tuirealm::AttrValue::Size(self.height())),
            tuirealm::Attribute::Custom(SHOW_FULL) => Some(tuirealm::AttrValue::Flag(matches!(
                self.state,
                HelpState::Full
            ))),
            _ => None,
        }
    }

    fn attr(&mut self, attr: tuirealm::Attribute, value: tuirealm::AttrValue) {
        if attr == tuirealm::Attribute::Custom(SHOW_FULL) {
            let show_full = value.as_flag().unwrap_or(true);
            let state = if show_full {
                HelpState::Full
            } else {
                HelpState::Short
            };

            self.state = state;
        }
    }

    fn state(&self) -> tuirealm::State {
        tuirealm::State::One(tuirealm::StateValue::Bool(matches!(
            self.state,
            HelpState::Short
        )))
    }

    fn perform(&mut self, _cmd: tuirealm::command::Cmd) -> tuirealm::command::CmdResult {
        // might actually implement `?` binding based on config?
        tuirealm::command::CmdResult::None
    }
}

impl<KeyId, T: HelpInfo<KeyId>> HelpWidget<KeyId, T> {
    pub fn new(info: T) -> Self {
        Self::from(Rc::new(RefCell::new(info)))
    }

    pub fn height(&self) -> u16 {
        match self.state {
            HelpState::Full => {
                let (_, rows) = self.help_info.borrow().full_help();
                rows as u16
            }
            HelpState::Short => 1,
        }
    }
}
