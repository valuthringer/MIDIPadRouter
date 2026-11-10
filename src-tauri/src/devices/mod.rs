//! Everything model-specific lives behind `DeviceProfile`.
//! Only the Launchpad Mini MK1 is implemented (v1 scope).

pub mod launchpad_mini_mk1;

use serde::{Deserialize, Serialize};

/// Number of physical keys handled by the router (8x8 pads + 8 scene + 8 top).
pub const KEY_COUNT: usize = 80;

/// Key index, `0..KEY_COUNT`. The order is device-defined.
pub type Key = usize;

/// How a key is addressed on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyKind {
    /// Note On/Off.
    Pad,
    /// Control Change.
    Cc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortMatch {
    Supported,
    /// Looks like a Launchpad, but another model/protocol.
    Unsupported,
    Other,
}

/// What a message sent by an application to its "virtual Launchpad" means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppMsg {
    /// Set one LED, or two for rapid-update messages.
    Led(Key, u8, Option<(Key, u8)>),
    /// Clear every LED of this application.
    Reset,
    /// Harmless global message, may be forwarded as-is.
    Forward,
    /// Layout/buffer/test/unknown message: never forwarded.
    Block,
}

pub trait DeviceProfile: Send + Sync {
    fn classify_port(&self, name: &str) -> PortMatch;
    fn key_to_midi(&self, key: Key) -> (KeyKind, u8);
    fn key_from_midi(&self, kind: KeyKind, number: u8) -> Option<Key>;
    /// Parse a message coming from the device: `(key, pressed)`.
    fn parse_input(&self, msg: &[u8]) -> Option<(Key, bool)>;
    /// Interpret a message from an application. `rapid` holds the rapid-update cursor.
    fn parse_app(&self, msg: &[u8], rapid: &mut Option<Key>) -> AppMsg;
    /// Message lighting one LED.
    fn led_msg(&self, key: Key, velocity: u8) -> [u8; 3];
    /// Clear the device and write every LED, through `send`.
    fn frame(&self, leds: &[u8; KEY_COUNT], send: &mut dyn FnMut(&[u8]));
    /// LED velocity used to show which target owns a key (target slot 0..5).
    fn preview_color(&self, slot: usize) -> u8;
}

/// The only device of v1.
pub static DEVICE: &dyn DeviceProfile = &launchpad_mini_mk1::LaunchpadMiniMk1;
