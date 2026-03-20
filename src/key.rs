use std::{
    borrow::Cow,
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use tuirealm::{command::Cmd, event::KeyEvent};

/// A key binding(s) and its help information
#[derive(Debug)]
pub struct Binding {
    /// All the keys that bind to the same action
    keys: Vec<KeyEvent>,
    help: Help,
    disabled: bool,
    cmd: Cmd,
}

/// Help information for bindings
#[derive(Debug)]
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

impl Binding {
    pub fn new(keys: impl IntoIterator<Item = impl Into<KeyEvent>>, help: Help) -> Self {
        Self {
            keys: keys.into_iter().map(|it| it.into()).collect(),
            help,
            disabled: false,
            cmd: Cmd::None,
        }
    }

    pub fn command(mut self, cmd: Cmd) -> Self {
        self.cmd = cmd;
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
pub struct KeyMap<KeyDescriptor: Ord> {
    map: BTreeMap<KeyDescriptor, Binding>,
}

impl<T: Ord, I: Into<BTreeMap<T, Binding>>> From<I> for KeyMap<T> {
    fn from(value: I) -> Self {
        Self { map: value.into() }
    }
}

impl<T: Ord> KeyMap<T> {
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

    /// Matches key event, returning the first msg
    ///
    /// NOTE: the order of checks can change over versions
    pub fn match_key_event(&self, ev: KeyEvent) -> Cmd {
        self.values()
            .filter(|binding| !binding.disabled)
            .find(|binding| binding.keys.contains(&ev))
            .map(|binding| binding.cmd)
            .unwrap_or(Cmd::None)
    }
}

impl<T: Ord> Deref for KeyMap<T> {
    type Target = BTreeMap<T, Binding>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<T: Ord> DerefMut for KeyMap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}
