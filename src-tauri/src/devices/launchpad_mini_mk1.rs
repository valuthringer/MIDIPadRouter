//! Novation Launchpad Mini MK1 (same protocol as Launchpad MK1 / S / Mini MK2).
//! Source: Novation "Launchpad Programmer's Reference", X-Y layout.
//!
//! Key index order (= rapid-update order of the reference):
//! 0..64 grid left-to-right, top-to-bottom; 64..72 scene buttons top-to-bottom;
//! 72..80 top buttons left-to-right.

use super::{AppMsg, DeviceProfile, Key, KeyKind, PortMatch, KEY_COUNT};

pub const NOTE_OFF: u8 = 0x80;
pub const NOTE_ON: u8 = 0x90;
pub const CC: u8 = 0xB0;
/// Note On on channel 3: rapid LED update, two LEDs per message.
pub const RAPID: u8 = 0x92;
/// Top buttons: CC 104..=111.
pub const TOP_CC_FIRST: u8 = 104;
/// `B0 00 00`: all LEDs off, X-Y layout, buffers and duty cycle back to default.
pub const RESET: [u8; 3] = [CC, 0, 0];
/// Double-buffer control byte `0b0010_0000 | copy<<4 | flash<<3 | update<<2 | display`.
/// Only "buffer 0, no copy" with flash off (0x20) or on (0x28) keeps our state coherent.
pub const BUFFER_DEFAULT: u8 = 0x20;
pub const BUFFER_FLASH: u8 = 0x28;

/// LED flags: normal display (copy + clear).
pub const FLAGS_NORMAL: u8 = 12;
/// LED flags: flashing (clear only: lit in the update buffer, off in the other one;
/// auto-flash `B0 00 28` alternates them).
#[allow(dead_code)] // protocol reference, used by tests
pub const FLAGS_FLASH: u8 = 8;
/// LED flags: double buffering (write update buffer only).
#[allow(dead_code)] // protocol reference, used by tests
pub const FLAGS_DOUBLE_BUFFER: u8 = 0;

/// Velocity = 16 * green + red + flags, green and red in 0..=3.
pub const fn led_velocity(green: u8, red: u8, flags: u8) -> u8 {
    16 * green + red + flags
}

pub const LED_OFF: u8 = led_velocity(0, 0, FLAGS_NORMAL);

/// Colours used to show key ownership on the hardware (green, red, amber, yellow, orange).
pub const PREVIEW_COLORS: [u8; 5] = [
    led_velocity(3, 0, FLAGS_NORMAL),
    led_velocity(0, 3, FLAGS_NORMAL),
    led_velocity(3, 3, FLAGS_NORMAL),
    led_velocity(3, 2, FLAGS_NORMAL),
    led_velocity(2, 3, FLAGS_NORMAL),
];

pub struct LaunchpadMiniMk1;

fn off_if_zero(v: u8) -> u8 {
    if v == 0 {
        LED_OFF
    } else {
        v
    }
}

impl DeviceProfile for LaunchpadMiniMk1 {
    fn classify_port(&self, name: &str) -> PortMatch {
        let n = name.to_lowercase();
        // MK3 and the RGB models use a different protocol and other port names.
        if n.contains("mk3") || n.contains("lpmini") || n.contains("lpx") || n.contains("lppro") {
            return PortMatch::Unsupported;
        }
        if n.contains("launchpad mini") {
            PortMatch::Supported
        } else if n.contains("launchpad") {
            PortMatch::Unsupported
        } else {
            PortMatch::Other
        }
    }

    fn key_to_midi(&self, key: Key) -> (KeyKind, u8) {
        let k = key as u8;
        match key {
            0..=63 => (KeyKind::Pad, 16 * (k / 8) + k % 8),
            64..=71 => (KeyKind::Pad, 16 * (k - 64) + 8),
            _ => (KeyKind::Cc, TOP_CC_FIRST + (k - 72)),
        }
    }

    fn key_from_midi(&self, kind: KeyKind, number: u8) -> Option<Key> {
        match kind {
            KeyKind::Pad => {
                let (row, col) = ((number >> 4) as usize, (number & 0x0F) as usize);
                match (row, col) {
                    (0..=7, 0..=7) => Some(row * 8 + col),
                    (0..=7, 8) => Some(64 + row),
                    _ => None,
                }
            }
            KeyKind::Cc => (TOP_CC_FIRST..TOP_CC_FIRST + 8)
                .contains(&number)
                .then(|| 72 + (number - TOP_CC_FIRST) as usize),
        }
    }

    fn parse_input(&self, msg: &[u8]) -> Option<(Key, bool)> {
        match *msg {
            [NOTE_ON, n, v] => Some((self.key_from_midi(KeyKind::Pad, n)?, v > 0)),
            [NOTE_OFF, n, _] => Some((self.key_from_midi(KeyKind::Pad, n)?, false)),
            [CC, c, v] => Some((self.key_from_midi(KeyKind::Cc, c)?, v > 0)),
            _ => None,
        }
    }

    fn parse_app(&self, msg: &[u8], rapid: &mut Option<Key>) -> AppMsg {
        // Any message other than a rapid update ends the rapid-update sequence.
        if msg.first() != Some(&RAPID) {
            *rapid = None;
        }
        match *msg {
            [NOTE_ON, n, v] => self
                .key_from_midi(KeyKind::Pad, n)
                .map_or(AppMsg::Block, |k| AppMsg::Led(k, v, None)),
            [NOTE_OFF, n, _] => self
                .key_from_midi(KeyKind::Pad, n)
                .map_or(AppMsg::Block, |k| AppMsg::Led(k, 0, None)),
            [CC, 0, 0] => AppMsg::Reset,
            [CC, 0, BUFFER_DEFAULT | BUFFER_FLASH] => AppMsg::Forward,
            // Layout (01/02), other buffer modes, LED test (7D..7F): blocked.
            [CC, 0, _] => AppMsg::Block,
            // Top-button LEDs; any other CC (e.g. duty cycle 30/31) is blocked.
            [CC, c, v] => self
                .key_from_midi(KeyKind::Cc, c)
                .map_or(AppMsg::Block, |k| AppMsg::Led(k, v, None)),
            [RAPID, a, b] => {
                let c = rapid.unwrap_or(0);
                *rapid = Some(c + 2);
                if c >= KEY_COUNT {
                    return AppMsg::Block; // overflow is ignored by the device too
                }
                AppMsg::Led(c, a, (c + 1 < KEY_COUNT).then_some((c + 1, b)))
            }
            // SysEx, other channels, malformed: blocked.
            _ => AppMsg::Block,
        }
    }

    fn led_msg(&self, key: Key, velocity: u8) -> [u8; 3] {
        let v = off_if_zero(velocity);
        match self.key_to_midi(key) {
            (KeyKind::Pad, n) => [NOTE_ON, n, v],
            (KeyKind::Cc, n) => [CC, n, v],
        }
    }

    fn frame(&self, leds: &[u8; KEY_COUNT], send: &mut dyn FnMut(&[u8])) {
        // Reset also restarts the rapid-update cursor at the top-left pad.
        send(&RESET);
        for [a, b] in leds.as_chunks::<2>().0 {
            send(&[RAPID, off_if_zero(*a), off_if_zero(*b)]);
        }
    }

    fn preview_color(&self, slot: usize) -> u8 {
        PREVIEW_COLORS[slot % PREVIEW_COLORS.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const D: LaunchpadMiniMk1 = LaunchpadMiniMk1;

    #[test]
    fn all_80_keys_round_trip() {
        let mut seen = std::collections::HashSet::new();
        for k in 0..KEY_COUNT {
            let (kind, n) = D.key_to_midi(k);
            assert!(seen.insert((kind, n)), "duplicate midi id for key {k}");
            assert_eq!(D.key_from_midi(kind, n), Some(k));
            // And through real messages.
            let msg = D.led_msg(k, 127);
            assert_eq!(D.parse_input(&msg), Some((k, true)));
        }
    }

    #[test]
    fn reference_numbers() {
        assert_eq!(D.key_to_midi(0), (KeyKind::Pad, 0));
        assert_eq!(D.key_to_midi(9), (KeyKind::Pad, 17)); // row 1, col 1
        assert_eq!(D.key_to_midi(63), (KeyKind::Pad, 119));
        let scene: Vec<u8> = (64..72).map(|k| D.key_to_midi(k).1).collect();
        assert_eq!(scene, [8, 24, 40, 56, 72, 88, 104, 120]);
        let top: Vec<u8> = (72..80).map(|k| D.key_to_midi(k).1).collect();
        assert_eq!(top, [104, 105, 106, 107, 108, 109, 110, 111]);
        assert_eq!(D.key_from_midi(KeyKind::Pad, 9), None); // column 9 does not exist
        assert_eq!(D.key_from_midi(KeyKind::Pad, 128), None);
        assert_eq!(D.key_from_midi(KeyKind::Cc, 103), None);
        assert_eq!(D.key_from_midi(KeyKind::Cc, 112), None);
    }

    #[test]
    fn input_press_and_release() {
        assert_eq!(D.parse_input(&[0x90, 0, 127]), Some((0, true)));
        assert_eq!(D.parse_input(&[0x90, 0, 0]), Some((0, false)));
        assert_eq!(D.parse_input(&[0x80, 8, 64]), Some((64, false)));
        assert_eq!(D.parse_input(&[0xB0, 111, 127]), Some((79, true)));
        assert_eq!(D.parse_input(&[0xB0, 111, 0]), Some((79, false)));
        assert_eq!(D.parse_input(&[0xF8]), None);
    }

    #[test]
    fn led_velocity_encoding() {
        assert_eq!(led_velocity(0, 3, FLAGS_NORMAL), 0x0F); // red full
        assert_eq!(led_velocity(3, 0, FLAGS_NORMAL), 0x3C); // green full
        assert_eq!(led_velocity(3, 3, FLAGS_NORMAL), 0x3F); // amber full
        assert_eq!(led_velocity(0, 3, FLAGS_FLASH), 0x0B); // red flashing
        assert_eq!(led_velocity(3, 0, FLAGS_DOUBLE_BUFFER), 0x30);
        assert_eq!(LED_OFF, 0x0C);
        assert_eq!(D.led_msg(0, 0), [0x90, 0, 0x0C]);
        assert_eq!(D.led_msg(72, 0x3C), [0xB0, 104, 0x3C]);
    }

    #[test]
    fn special_messages_are_blocked_or_interpreted() {
        let mut r = None;
        assert_eq!(D.parse_app(&RESET, &mut r), AppMsg::Reset);
        assert_eq!(D.parse_app(&[0xB0, 0, 1], &mut r), AppMsg::Block); // X-Y layout
        assert_eq!(D.parse_app(&[0xB0, 0, 2], &mut r), AppMsg::Block); // drum rack
        assert_eq!(D.parse_app(&[0xB0, 0, 0x31], &mut r), AppMsg::Block); // buffer swap
        assert_eq!(D.parse_app(&[0xB0, 0, 0x7F], &mut r), AppMsg::Block); // LED test
        assert_eq!(D.parse_app(&[0xB0, 0x1E, 0x02], &mut r), AppMsg::Block); // duty cycle
        assert_eq!(D.parse_app(&[0xB0, 0, 0x28], &mut r), AppMsg::Forward); // auto flash
        assert_eq!(D.parse_app(&[0xF0, 0, 0x20], &mut r), AppMsg::Block);
        assert_eq!(D.parse_app(&[0x91, 0, 0x3C], &mut r), AppMsg::Block); // other channel
        assert_eq!(D.parse_app(&[0x80, 17, 0], &mut r), AppMsg::Led(9, 0, None));
    }

    #[test]
    fn rapid_update_walks_all_keys_then_restarts() {
        let mut r = None;
        for i in 0..40 {
            let k = i * 2;
            assert_eq!(
                D.parse_app(&[RAPID, 1, 2], &mut r),
                AppMsg::Led(k, 1, Some((k + 1, 2)))
            );
        }
        assert_eq!(D.parse_app(&[RAPID, 1, 2], &mut r), AppMsg::Block); // overflow
        D.parse_app(&[0x90, 0, 0], &mut r); // any other message restarts
        assert_eq!(
            D.parse_app(&[RAPID, 5, 6], &mut r),
            AppMsg::Led(0, 5, Some((1, 6)))
        );
    }

    #[test]
    fn frame_is_reset_then_40_rapid_messages() {
        let mut leds = [0u8; KEY_COUNT];
        leds[0] = 0x3C;
        let mut sent = Vec::new();
        D.frame(&leds, &mut |m| sent.push(m.to_vec()));
        assert_eq!(sent.len(), 41);
        assert_eq!(sent[0], RESET);
        assert_eq!(sent[1], [RAPID, 0x3C, LED_OFF]);
    }

    #[test]
    fn port_names() {
        assert_eq!(D.classify_port("Launchpad Mini"), PortMatch::Supported);
        assert_eq!(D.classify_port("2- Launchpad Mini"), PortMatch::Supported);
        assert_eq!(D.classify_port("LPMiniMK3 MIDI"), PortMatch::Unsupported);
        assert_eq!(
            D.classify_port("Launchpad Mini MK3 LPMiniMK3 MIDI Out"),
            PortMatch::Unsupported
        );
        assert_eq!(D.classify_port("Launchpad MK2"), PortMatch::Unsupported);
        assert_eq!(D.classify_port("loopMIDI Port"), PortMatch::Other);
    }
}
