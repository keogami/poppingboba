use std::{
    borrow::Cow,
    cell::RefCell,
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::help::{Help, HelpInfo};

/// A key binding(s) and its help information
pub struct Binding<Msg = ()> {
    /// All the keys that bind to the same action
    keys: Vec<KeyEvent>,
    help: Option<Help>,
    disabled: bool,
    msg: Option<Msg>,
}

pub trait IntoBinding {
    fn into_binding<Msg>(self) -> Binding<Msg>;
}

impl IntoBinding for KeyCode {
    fn into_binding<Msg>(self) -> Binding<Msg> {
        Binding::new([KeyEvent::from(self)])
    }
}

impl<const N: usize> IntoBinding for [KeyCode; N] {
    fn into_binding<Msg>(self) -> Binding<Msg> {
        Binding::new(self.map(KeyEvent::from))
    }
}

impl IntoBinding for KeyEvent {
    fn into_binding<Msg>(self) -> Binding<Msg> {
        Binding::new([self])
    }
}

impl<const N: usize> IntoBinding for [KeyEvent; N] {
    fn into_binding<Msg>(self) -> Binding<Msg> {
        Binding::new(self)
    }
}

impl<Msg> Binding<Msg> {
    pub fn new(keys: impl IntoIterator<Item = impl Into<KeyEvent>>) -> Self {
        Self {
            keys: keys.into_iter().map(|it| it.into()).collect(),
            help: None,
            disabled: false,
            msg: None,
        }
    }

    pub fn new_with_help(
        keys: impl IntoIterator<Item = impl Into<KeyEvent>>,
        help: impl Into<Help>,
    ) -> Self {
        Self::new(keys).help(help)
    }
}

impl<Msg> Binding<Msg> {
    pub fn message(mut self, msg: Msg) -> Self {
        self.msg = Some(msg);
        self
    }

    pub fn help(mut self, help: impl Into<Help>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn enabled(mut self) -> Self {
        self.disabled = false;
        self
    }
}

/// A map of key bindings
pub struct KeyMap<KeyId: Clone + 'static, Msg = ()> {
    map: BTreeMap<KeyId, Binding<Msg>>,
    short: Cow<'static, [KeyId]>,
    full: (Cow<'static, [KeyId]>, usize),
}

pub type ShareableKeyMap<T, Msg = ()> = Rc<RefCell<KeyMap<T, Msg>>>;

impl<T, Msg, I> From<I> for KeyMap<T, Msg>
where
    T: Ord + Clone,
    I: Into<BTreeMap<T, Binding<Msg>>>,
{
    fn from(value: I) -> Self {
        Self {
            map: value.into(),
            short: Default::default(),
            full: Default::default(),
        }
    }
}

impl<T, Msg> KeyMap<T, Msg>
where
    T: Ord + Clone,
{
    /// disables the given key binding
    ///
    /// noop if binding doesn't exist
    pub fn disable(&mut self, key: &T) {
        let Some(key) = self.get_mut(key) else {
            return;
        };
        key.disabled = true;
    }

    /// enables the given key binding, noop if
    ///
    /// noop if binding doesn't exist
    pub fn enable(&mut self, key: &T) {
        let Some(key) = self.get_mut(key) else {
            return;
        };
        key.disabled = false;
    }

    pub fn shareable(self) -> ShareableKeyMap<T, Msg> {
        Rc::new(RefCell::new(self))
    }
}

impl<KeyId: Clone, Msg> KeyMap<KeyId, Msg> {
    pub fn short_help(mut self, short: impl Into<Cow<'static, [KeyId]>>) -> Self {
        self.short = short.into();
        self
    }

    pub fn full_help(mut self, rows: usize, keys: impl Into<Cow<'static, [KeyId]>>) -> Self {
        self.full = (keys.into(), rows);
        self
    }
}

impl<T, Msg> KeyMap<T, Msg>
where
    T: Ord + Clone,
    Msg: Clone,
{
    /// Matches key event, returning the first msg
    ///
    /// NOTE: the order of checks can change over versions
    pub fn match_key_event(&self, ev: KeyEvent) -> Option<Msg> {
        self.values()
            .filter(|binding| !binding.disabled)
            .find(|binding| binding.keys.iter().any(|k| k.code == ev.code && k.modifiers == ev.modifiers))
            .and_then(|binding| binding.msg.clone())
    }
}

impl<T, Msg> Deref for KeyMap<T, Msg>
where
    T: Ord + Clone,
{
    type Target = BTreeMap<T, Binding<Msg>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<KeyId, Msg> HelpInfo<KeyId> for KeyMap<KeyId, Msg>
where
    KeyId: Clone + Ord,
{
    fn short_help(&self) -> &[KeyId] {
        &self.short
    }

    fn full_help(&self) -> (&[KeyId], usize) {
        (&self.full.0, self.full.1)
    }

    fn help(&self, key_id: &KeyId) -> &Help {
        self.map
            .get(key_id)
            .and_then(|k| k.help.as_ref())
            .expect("The user supplies valid keys")
    }
}

impl<T, Msg> DerefMut for KeyMap<T, Msg>
where
    T: Ord + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}
