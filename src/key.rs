use std::{
    borrow::Cow,
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use tui_realm_stdlib::Phantom;
use tuirealm::{Component, Event, MockComponent, Sub, event::KeyEvent};

/// A key binding(s) and its help information
pub struct Binding<Msg> {
    /// All the keys that bind to the same action
    keys: Vec<KeyEvent>,
    help: Help,
    disabled: bool,
    msg: Msg,
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

impl<Msg: Default> Binding<Msg> {
    pub fn new(keys: impl IntoIterator<Item = impl Into<KeyEvent>>, help: Help) -> Self {
        Self {
            keys: keys.into_iter().map(|it| it.into()).collect(),
            help,
            disabled: false,
            msg: Msg::default(),
        }
    }
}

impl<Msg> Binding<Msg> {
    pub fn message(mut self, msg: Msg) -> Self {
        self.msg = msg;
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
pub struct KeyMap<KeyDescriptor, Msg = ()> {
    map: BTreeMap<KeyDescriptor, Binding<Msg>>,
}

impl<T: Ord, Msg, I: Into<BTreeMap<T, Binding<Msg>>>> From<I> for KeyMap<T, Msg> {
    fn from(value: I) -> Self {
        Self { map: value.into() }
    }
}

impl<T: Ord, Msg: Clone> KeyMap<T, Msg> {
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
    pub fn match_key_event(&self, ev: KeyEvent) -> Option<Msg> {
        self.values()
            .filter(|binding| !binding.disabled)
            .find(|binding| binding.keys.contains(&ev))
            .map(|binding| binding.msg.clone())
    }
}

impl<T: Ord, Msg> Deref for KeyMap<T, Msg> {
    type Target = BTreeMap<T, Binding<Msg>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<T: Ord, Msg> DerefMut for KeyMap<T, Msg> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

pub trait AsKeyMap: AsRef<KeyMap<Self::T, Self::Msg>> {
    type T;
    type Msg;
}

#[derive(MockComponent)]
pub struct KeyMapListener<K> {
    component: Phantom,
    key_map: K,
}

impl<K, UserEvent> Component<K::Msg, UserEvent> for KeyMapListener<K>
where
    UserEvent: PartialEq + Eq + Clone,
    K: AsKeyMap,
    K::Msg: PartialEq + Clone,
    K::T: Ord,
{
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<K::Msg> {
        match ev {
            Event::Keyboard(ev) => self.key_map.as_ref().match_key_event(ev),
            _ => None,
        }
    }
}

impl<K> KeyMapListener<K>
where
    K: AsKeyMap,
{
    pub fn new(key_map: K) -> Self {
        Self {
            key_map,
            component: Phantom::default(),
        }
    }
}
