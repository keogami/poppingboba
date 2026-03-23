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
            .find(|binding| binding.keys.contains(&ev))
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
        todo!()
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

pub struct KeyMapListener<T, Msg>
where
    T: Clone + 'static,
{
    component: Phantom,
    key_map: ShareableKeyMap<T, Msg>,
}

impl<T, Msg> MockComponent for KeyMapListener<T, Msg>
where
    T: Clone + 'static,
{
    fn view(&mut self, frame: &mut tuirealm::Frame, area: tuirealm::ratatui::prelude::Rect) {
        self.component.view(frame, area);
    }

    fn query(&self, attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        self.component.query(attr)
    }

    fn attr(&mut self, attr: tuirealm::Attribute, value: tuirealm::AttrValue) {
        self.component.attr(attr, value);
    }

    fn state(&self) -> tuirealm::State {
        self.component.state()
    }

    fn perform(&mut self, cmd: tuirealm::command::Cmd) -> tuirealm::command::CmdResult {
        self.component.perform(cmd)
    }
}

impl<T, Msg, UserEvent> Component<Msg, UserEvent> for KeyMapListener<T, Msg>
where
    UserEvent: PartialEq + Eq + Clone,
    T: Ord + Clone,
    Msg: PartialEq + Clone,
{
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(ev) => self.key_map.borrow().match_key_event(ev),
            _ => None,
        }
    }
}

impl<T, Msg> KeyMapListener<T, Msg>
where
    T: Clone,
{
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
