use std::{borrow::Cow, cell::RefCell, collections::BTreeMap, marker::PhantomData, rc::Rc};

use tuirealm::{
    MockComponent,
    ratatui::{
        text::{Line, Span},
        widgets::Widget,
    },
};

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
    _marker: PhantomData<KeyId>,
}

impl<KeyId, Info: HelpInfo<KeyId>> From<Rc<RefCell<Info>>> for HelpWidget<KeyId, Info> {
    fn from(value: Rc<RefCell<Info>>) -> Self {
        Self {
            help_info: value.clone(),
            state: HelpState::Short,
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
                // noop
            }
            HelpState::Short => {
                let separator = SHORT_SEP;
                let ellipsis = format!(" {}", ELLIPSIS);
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
                        spans.push(Span::raw(separator));
                    }

                    spans.push(Span::raw(help_info.help(item).key.clone()));
                    spans.push(Span::raw(" "));
                    spans.push(Span::raw(help_info.help(item).desc.clone()));
                }
                if needs_ellipsis {
                    spans.push(Span::raw(ellipsis));
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
            _ => None,
        }
    }

    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {
        // noop
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
        Self {
            help_info: Rc::new(RefCell::new(info)),
            state: HelpState::Short,
            _marker: PhantomData,
        }
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
