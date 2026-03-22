use std::{
    borrow::Cow,
    cell::RefCell,
    collections::BTreeMap,
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use tui_realm_stdlib::Phantom;
use tuirealm::{
    Component, Event, MockComponent, Sub, SubClause, SubEventClause,
    event::{Key, KeyEvent},
};

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

impl IntoBinding for Key {
    fn into_binding<Msg>(self) -> Binding<Msg> {
        Binding::new([self])
    }
}

impl<const N: usize> IntoBinding for [Key; N] {
    fn into_binding<Msg>(self) -> Binding<Msg> {
        Binding::new(self)
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

impl<Key: Into<Cow<'static, str>>, Desc: Into<Cow<'static, str>>> From<(Key, Desc)> for Help {
    fn from((key, desc): (Key, Desc)) -> Self {
        Self {
            key: key.into(),
            desc: desc.into(),
        }
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
pub struct KeyMap<KeyDescriptor, Msg = ()> {
    map: BTreeMap<KeyDescriptor, Binding<Msg>>,
}

pub type ShareableKeyMap<T, Msg = ()> = Rc<RefCell<KeyMap<T, Msg>>>;

impl<T: Ord, Msg, I: Into<BTreeMap<T, Binding<Msg>>>> From<I> for KeyMap<T, Msg> {
    fn from(value: I) -> Self {
        Self { map: value.into() }
    }
}

impl<T: Ord, Msg> KeyMap<T, Msg> {
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

impl<T: Ord, Msg: Clone> KeyMap<T, Msg> {
    /// Matches key event, returning the first msg
    ///
    /// NOTE: the order of checks can change over versions
    pub fn match_key_event(&self, ev: KeyEvent) -> Option<Msg> {
        self.values()
            .filter(|binding| !binding.disabled)
            .find(|binding| binding.keys.contains(&ev))
            .and_then(|binding| binding.msg.clone())
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

#[derive(MockComponent)]
pub struct KeyMapListener<T, Msg> {
    component: Phantom,
    key_map: ShareableKeyMap<T, Msg>,
}

impl<T, Msg, UserEvent> Component<Msg, UserEvent> for KeyMapListener<T, Msg>
where
    UserEvent: PartialEq + Eq + Clone,
    T: Ord,
    Msg: PartialEq + Clone,
{
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(ev) => self.key_map.borrow().match_key_event(ev),
            _ => None,
        }
    }
}

impl<T, Msg> KeyMapListener<T, Msg> {
    pub fn new<Id, UserEvent>(key_map: ShareableKeyMap<T, Msg>) -> (Self, Vec<Sub<Id, UserEvent>>)
    where
        Id: Eq + PartialEq + Clone + Hash,
        UserEvent: Eq + PartialEq + Clone,
    {
        (
            Self {
                key_map,
                component: Phantom::default(),
            },
            vec![Sub::new(SubEventClause::Any, SubClause::Always)],
        )
    }
}
