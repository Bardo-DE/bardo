//! xkb keysym -> `keyboard_types::Key`: the logical key a press means under the current layout.
//!
//! The table is adapted from winit 0.30.13, `src/platform_impl/linux/common/xkb/keymap.rs`
//! (`keysym_to_key`, `keysym_location`), Copyright the winit contributors, licensed under the
//! Apache License 2.0 (<https://www.apache.org/licenses/LICENSE-2.0>). Changes from the original:
//! it targets keyboard-types 0.8's `Key`/`Location` instead of winit's `NamedKey`/`KeyLocation`;
//! the Super keys map to `Key::Named(NamedKey::Meta)` (as in winit), which our desktop treats as the Command
//! key and which pairs with `Modifiers::META`; `space` is `Key::Character(" ")`,
//! because keyboard-types has no named Space; commented-out rows of the original are omitted.
//! FINDINGS.md has the licence note.

use keyboard_types::{Key, Location, NamedKey};
use smithay_client_toolkit::seat::keyboard::Keysym;

/// The named key for `keysym`, or `None` when the keysym stands for text (or for nothing we
/// name).
#[expect(clippy::too_many_lines, reason = "a table: one arm per keysym")]
pub(crate) fn named_key(keysym: Keysym) -> Option<Key> {
    let key = match keysym {
        // TTY function keys
        Keysym::BackSpace => Key::Named(NamedKey::Backspace),
        Keysym::Tab | Keysym::KP_Tab | Keysym::ISO_Left_Tab => Key::Named(NamedKey::Tab),
        Keysym::Clear => Key::Named(NamedKey::Clear),
        Keysym::Return | Keysym::KP_Enter | Keysym::ISO_Enter | Keysym::_3270_Enter => {
            Key::Named(NamedKey::Enter)
        }
        Keysym::Pause | Keysym::Break => Key::Named(NamedKey::Pause),
        Keysym::Scroll_Lock => Key::Named(NamedKey::ScrollLock),
        Keysym::Sys_Req | Keysym::Print | Keysym::_3270_PrintScreen => {
            Key::Named(NamedKey::PrintScreen)
        }
        Keysym::Escape => Key::Named(NamedKey::Escape),
        Keysym::Delete | Keysym::KP_Delete => Key::Named(NamedKey::Delete),

        // IME keys
        Keysym::Multi_key => Key::Named(NamedKey::Compose),
        Keysym::Codeinput => Key::Named(NamedKey::CodeInput),
        Keysym::SingleCandidate => Key::Named(NamedKey::SingleCandidate),
        Keysym::MultipleCandidate => Key::Named(NamedKey::AllCandidates),
        Keysym::PreviousCandidate => Key::Named(NamedKey::PreviousCandidate),

        // Japanese keys
        Keysym::Kanji => Key::Named(NamedKey::KanjiMode),
        Keysym::Muhenkan => Key::Named(NamedKey::NonConvert),
        Keysym::Henkan_Mode => Key::Named(NamedKey::Convert),
        Keysym::Romaji => Key::Named(NamedKey::Romaji),
        Keysym::Hiragana => Key::Named(NamedKey::Hiragana),
        Keysym::Hiragana_Katakana => Key::Named(NamedKey::HiraganaKatakana),
        Keysym::Zenkaku => Key::Named(NamedKey::Zenkaku),
        Keysym::Hankaku => Key::Named(NamedKey::Hankaku),
        Keysym::Zenkaku_Hankaku => Key::Named(NamedKey::ZenkakuHankaku),
        Keysym::Kana_Lock | Keysym::Kana_Shift => Key::Named(NamedKey::KanaMode),
        Keysym::Eisu_Shift | Keysym::Eisu_toggle => Key::Named(NamedKey::Alphanumeric),

        // Cursor control & motion
        Keysym::Home | Keysym::KP_Home => Key::Named(NamedKey::Home),
        Keysym::Left | Keysym::KP_Left => Key::Named(NamedKey::ArrowLeft),
        Keysym::Up | Keysym::KP_Up => Key::Named(NamedKey::ArrowUp),
        Keysym::Right | Keysym::KP_Right => Key::Named(NamedKey::ArrowRight),
        Keysym::Down | Keysym::KP_Down => Key::Named(NamedKey::ArrowDown),
        Keysym::Page_Up | Keysym::KP_Page_Up => Key::Named(NamedKey::PageUp),
        Keysym::Page_Down | Keysym::KP_Page_Down => Key::Named(NamedKey::PageDown),
        Keysym::End | Keysym::KP_End => Key::Named(NamedKey::End),

        // Misc. functions
        Keysym::Select => Key::Named(NamedKey::Select),
        Keysym::Execute => Key::Named(NamedKey::Execute),
        Keysym::Insert | Keysym::KP_Insert => Key::Named(NamedKey::Insert),
        Keysym::Undo => Key::Named(NamedKey::Undo),
        Keysym::Redo => Key::Named(NamedKey::Redo),
        Keysym::Menu => Key::Named(NamedKey::ContextMenu),
        Keysym::Find => Key::Named(NamedKey::Find),
        Keysym::Cancel => Key::Named(NamedKey::Cancel),
        Keysym::Help => Key::Named(NamedKey::Help),
        Keysym::Mode_switch => Key::Named(NamedKey::ModeChange),
        Keysym::Num_Lock => Key::Named(NamedKey::NumLock),

        // Keypad keys
        Keysym::KP_F1 | Keysym::F1 => Key::Named(NamedKey::F1),
        Keysym::KP_F2 | Keysym::F2 => Key::Named(NamedKey::F2),
        Keysym::KP_F3 | Keysym::F3 => Key::Named(NamedKey::F3),
        Keysym::KP_F4 | Keysym::F4 => Key::Named(NamedKey::F4),

        // Function keys
        Keysym::F5 => Key::Named(NamedKey::F5),
        Keysym::F6 => Key::Named(NamedKey::F6),
        Keysym::F7 => Key::Named(NamedKey::F7),
        Keysym::F8 => Key::Named(NamedKey::F8),
        Keysym::F9 => Key::Named(NamedKey::F9),
        Keysym::F10 => Key::Named(NamedKey::F10),
        Keysym::F11 => Key::Named(NamedKey::F11),
        Keysym::F12 => Key::Named(NamedKey::F12),
        Keysym::F13 => Key::Named(NamedKey::F13),
        Keysym::F14 => Key::Named(NamedKey::F14),
        Keysym::F15 => Key::Named(NamedKey::F15),
        Keysym::F16 => Key::Named(NamedKey::F16),
        Keysym::F17 => Key::Named(NamedKey::F17),
        Keysym::F18 => Key::Named(NamedKey::F18),
        Keysym::F19 => Key::Named(NamedKey::F19),
        Keysym::F20 => Key::Named(NamedKey::F20),
        Keysym::F21 => Key::Named(NamedKey::F21),
        Keysym::F22 => Key::Named(NamedKey::F22),
        Keysym::F23 => Key::Named(NamedKey::F23),
        Keysym::F24 => Key::Named(NamedKey::F24),
        Keysym::F25 => Key::Named(NamedKey::F25),
        Keysym::F26 => Key::Named(NamedKey::F26),
        Keysym::F27 => Key::Named(NamedKey::F27),
        Keysym::F28 => Key::Named(NamedKey::F28),
        Keysym::F29 => Key::Named(NamedKey::F29),
        Keysym::F30 => Key::Named(NamedKey::F30),
        Keysym::F31 => Key::Named(NamedKey::F31),
        Keysym::F32 => Key::Named(NamedKey::F32),
        Keysym::F33 => Key::Named(NamedKey::F33),
        Keysym::F34 => Key::Named(NamedKey::F34),
        Keysym::F35 => Key::Named(NamedKey::F35),

        // Modifiers
        Keysym::Shift_L | Keysym::Shift_R => Key::Named(NamedKey::Shift),
        Keysym::Control_L | Keysym::Control_R => Key::Named(NamedKey::Control),
        Keysym::Caps_Lock => Key::Named(NamedKey::CapsLock),
        Keysym::Alt_L | Keysym::Alt_R => Key::Named(NamedKey::Alt),
        Keysym::Super_L | Keysym::Super_R => Key::Named(NamedKey::Meta),

        // XKB function and modifier keys
        Keysym::ISO_Level3_Shift | Keysym::ISO_Level3_Latch | Keysym::ISO_Level3_Lock => {
            Key::Named(NamedKey::AltGraph)
        }
        Keysym::ISO_Next_Group => Key::Named(NamedKey::GroupNext),
        Keysym::ISO_Prev_Group => Key::Named(NamedKey::GroupPrevious),
        Keysym::ISO_First_Group => Key::Named(NamedKey::GroupFirst),
        Keysym::ISO_Last_Group => Key::Named(NamedKey::GroupLast),

        // 3270 terminal keys
        Keysym::_3270_EraseEOF => Key::Named(NamedKey::EraseEof),
        Keysym::_3270_Attn => Key::Named(NamedKey::Attn),
        Keysym::_3270_Play => Key::Named(NamedKey::Play),
        Keysym::_3270_ExSelect => Key::Named(NamedKey::ExSel),
        Keysym::_3270_CursorSelect => Key::Named(NamedKey::CrSel),

        // XFree86: backlight
        Keysym::XF86_MonBrightnessUp | Keysym::SUN_VideoRaiseBrightness => {
            Key::Named(NamedKey::BrightnessUp)
        }
        Keysym::XF86_MonBrightnessDown | Keysym::SUN_VideoLowerBrightness => {
            Key::Named(NamedKey::BrightnessDown)
        }

        // XFree86: "Internet"
        Keysym::XF86_Standby | Keysym::XF86_Sleep | Keysym::XF86_Suspend => {
            Key::Named(NamedKey::Standby)
        }
        Keysym::XF86_AudioLowerVolume | Keysym::SUN_AudioLowerVolume => {
            Key::Named(NamedKey::AudioVolumeDown)
        }
        Keysym::XF86_AudioRaiseVolume | Keysym::SUN_AudioRaiseVolume => {
            Key::Named(NamedKey::AudioVolumeUp)
        }
        Keysym::XF86_AudioPlay => Key::Named(NamedKey::MediaPlay),
        Keysym::XF86_AudioStop => Key::Named(NamedKey::MediaStop),
        Keysym::XF86_AudioPrev => Key::Named(NamedKey::MediaTrackPrevious),
        Keysym::XF86_AudioNext => Key::Named(NamedKey::MediaTrackNext),
        Keysym::XF86_HomePage => Key::Named(NamedKey::BrowserHome),
        Keysym::XF86_Mail => Key::Named(NamedKey::LaunchMail),
        Keysym::XF86_Search => Key::Named(NamedKey::BrowserSearch),
        Keysym::XF86_AudioRecord => Key::Named(NamedKey::MediaRecord),

        // XFree86: PDA
        Keysym::XF86_Calculator | Keysym::XF86_Calculater => {
            Key::Named(NamedKey::LaunchApplication2)
        }
        Keysym::XF86_Calendar => Key::Named(NamedKey::LaunchCalendar),
        Keysym::XF86_PowerDown | Keysym::XF86_PowerOff => Key::Named(NamedKey::Power),

        // XFree86: more "Internet"
        Keysym::XF86_Back => Key::Named(NamedKey::BrowserBack),
        Keysym::XF86_Forward => Key::Named(NamedKey::BrowserForward),
        Keysym::XF86_Refresh | Keysym::XF86_Reload => Key::Named(NamedKey::BrowserRefresh),
        Keysym::XF86_WakeUp => Key::Named(NamedKey::WakeUp),
        Keysym::XF86_Eject => Key::Named(NamedKey::Eject),
        Keysym::XF86_ScreenSaver => Key::Named(NamedKey::LaunchScreenSaver),
        Keysym::XF86_WWW => Key::Named(NamedKey::LaunchWebBrowser),
        Keysym::XF86_Favorites | Keysym::XF86_MySites => Key::Named(NamedKey::BrowserFavorites),
        Keysym::XF86_AudioPause => Key::Named(NamedKey::MediaPause),
        Keysym::XF86_MyComputer => Key::Named(NamedKey::LaunchApplication1),
        Keysym::XF86_AudioRewind => Key::Named(NamedKey::MediaRewind),
        // libxkbcommon's own spelling, kept alongside Calculator.
        Keysym::XF86_Close => Key::Named(NamedKey::Close),
        Keysym::XF86_Copy | Keysym::SUN_Copy => Key::Named(NamedKey::Copy),
        Keysym::XF86_Cut | Keysym::SUN_Cut => Key::Named(NamedKey::Cut),
        Keysym::XF86_Excel => Key::Named(NamedKey::LaunchSpreadsheet),
        Keysym::XF86_LogOff => Key::Named(NamedKey::LogOff),
        Keysym::XF86_New => Key::Named(NamedKey::New),
        Keysym::XF86_Open | Keysym::SUN_Open => Key::Named(NamedKey::Open),
        Keysym::XF86_Paste | Keysym::SUN_Paste => Key::Named(NamedKey::Paste),
        Keysym::XF86_Phone => Key::Named(NamedKey::LaunchPhone),
        Keysym::XF86_Reply => Key::Named(NamedKey::MailReply),
        Keysym::XF86_Save => Key::Named(NamedKey::Save),
        Keysym::XF86_Send => Key::Named(NamedKey::MailSend),
        Keysym::XF86_Spell => Key::Named(NamedKey::SpellCheck),
        Keysym::XF86_SplitScreen => Key::Named(NamedKey::SplitScreenToggle),
        Keysym::XF86_Video => Key::Named(NamedKey::LaunchMediaPlayer),
        Keysym::XF86_Word => Key::Named(NamedKey::LaunchWordProcessor),
        Keysym::XF86_ZoomIn => Key::Named(NamedKey::ZoomIn),
        Keysym::XF86_ZoomOut => Key::Named(NamedKey::ZoomOut),
        Keysym::XF86_WebCam => Key::Named(NamedKey::LaunchWebCam),
        Keysym::XF86_MailForward => Key::Named(NamedKey::MailForward),
        Keysym::XF86_Music => Key::Named(NamedKey::LaunchMusicPlayer),
        Keysym::XF86_AudioForward => Key::Named(NamedKey::MediaFastForward),
        Keysym::XF86_AudioRandomPlay => Key::Named(NamedKey::RandomToggle),
        Keysym::XF86_Subtitle => Key::Named(NamedKey::Subtitle),
        Keysym::XF86_AudioCycleTrack => Key::Named(NamedKey::MediaAudioTrack),
        Keysym::XF86_Hibernate => Key::Named(NamedKey::Hibernate),
        Keysym::XF86_AudioMute | Keysym::SUN_AudioMute => Key::Named(NamedKey::AudioVolumeMute),
        Keysym::XF86_Next_VMode => Key::Named(NamedKey::VideoModeNext),

        // Sun
        _ => return None,
    };
    Some(key)
}

/// The logical key for a press: the named key if there is one, else the text the press
/// produced, else the keysym's own character, else `Unidentified`.
///
/// `text` is xkb's UTF-8 for the press (sctk's `KeyEvent::utf8`). Control characters in it
/// (Ctrl+C yields U+0003) are not text, so they fall through to the keysym's character (`c`),
/// which is what a shortcut matcher wants to see.
pub(crate) fn logical_key(keysym: Keysym, text: Option<&str>) -> Key {
    if let Some(key) = named_key(keysym) {
        return key;
    }
    let printable = text.filter(|t| !t.is_empty() && !t.chars().any(char::is_control));
    match (printable, keysym.key_char()) {
        (Some(t), _) => Key::Character(t.to_owned()),
        (None, Some(c)) if !c.is_control() => Key::Character(c.to_string()),
        _ => Key::Named(NamedKey::Unidentified),
    }
}

/// Where on the keyboard the key that produced `keysym` sits.
pub(crate) fn location(keysym: Keysym) -> Location {
    match keysym {
        Keysym::Shift_L
        | Keysym::Control_L
        | Keysym::Meta_L
        | Keysym::Alt_L
        | Keysym::Super_L
        | Keysym::Hyper_L => Location::Left,
        Keysym::Shift_R
        | Keysym::Control_R
        | Keysym::Meta_R
        | Keysym::Alt_R
        | Keysym::Super_R
        | Keysym::Hyper_R => Location::Right,
        Keysym::KP_0
        | Keysym::KP_1
        | Keysym::KP_2
        | Keysym::KP_3
        | Keysym::KP_4
        | Keysym::KP_5
        | Keysym::KP_6
        | Keysym::KP_7
        | Keysym::KP_8
        | Keysym::KP_9
        | Keysym::KP_Space
        | Keysym::KP_Tab
        | Keysym::KP_Enter
        | Keysym::KP_F1
        | Keysym::KP_F2
        | Keysym::KP_F3
        | Keysym::KP_F4
        | Keysym::KP_Home
        | Keysym::KP_Left
        | Keysym::KP_Up
        | Keysym::KP_Right
        | Keysym::KP_Down
        | Keysym::KP_Page_Up
        | Keysym::KP_Page_Down
        | Keysym::KP_End
        | Keysym::KP_Begin
        | Keysym::KP_Insert
        | Keysym::KP_Delete
        | Keysym::KP_Equal
        | Keysym::KP_Multiply
        | Keysym::KP_Add
        | Keysym::KP_Separator
        | Keysym::KP_Subtract
        | Keysym::KP_Decimal
        | Keysym::KP_Divide => Location::Numpad,
        _ => Location::Standard,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(group: &str, rows: &[(Keysym, Key)]) {
        for (keysym, want) in rows {
            assert_eq!(
                named_key(*keysym).as_ref(),
                Some(want),
                "{group}: {keysym:?}"
            );
        }
    }

    #[test]
    fn tty_and_cursor_keys() {
        check(
            "tty",
            &[
                (Keysym::BackSpace, Key::Named(NamedKey::Backspace)),
                (Keysym::Tab, Key::Named(NamedKey::Tab)),
                (Keysym::Return, Key::Named(NamedKey::Enter)),
                (Keysym::Escape, Key::Named(NamedKey::Escape)),
                (Keysym::Delete, Key::Named(NamedKey::Delete)),
                (Keysym::Sys_Req, Key::Named(NamedKey::PrintScreen)),
            ],
        );
        check(
            "cursor",
            &[
                (Keysym::Home, Key::Named(NamedKey::Home)),
                (Keysym::Left, Key::Named(NamedKey::ArrowLeft)),
                (Keysym::Up, Key::Named(NamedKey::ArrowUp)),
                (Keysym::Right, Key::Named(NamedKey::ArrowRight)),
                (Keysym::Down, Key::Named(NamedKey::ArrowDown)),
                (Keysym::Page_Up, Key::Named(NamedKey::PageUp)),
                (Keysym::Page_Down, Key::Named(NamedKey::PageDown)),
                (Keysym::End, Key::Named(NamedKey::End)),
            ],
        );
    }

    #[test]
    fn ime_and_japanese_keys() {
        check(
            "ime",
            &[
                (Keysym::Multi_key, Key::Named(NamedKey::Compose)),
                (
                    Keysym::MultipleCandidate,
                    Key::Named(NamedKey::AllCandidates),
                ),
                (Keysym::Henkan_Mode, Key::Named(NamedKey::Convert)),
                (Keysym::Muhenkan, Key::Named(NamedKey::NonConvert)),
                (
                    Keysym::Zenkaku_Hankaku,
                    Key::Named(NamedKey::ZenkakuHankaku),
                ),
                (Keysym::Eisu_toggle, Key::Named(NamedKey::Alphanumeric)),
            ],
        );
    }

    #[test]
    fn misc_keypad_and_function_keys() {
        check(
            "misc",
            &[
                (Keysym::Menu, Key::Named(NamedKey::ContextMenu)),
                (Keysym::Break, Key::Named(NamedKey::Pause)),
                (Keysym::Mode_switch, Key::Named(NamedKey::ModeChange)),
                (Keysym::Num_Lock, Key::Named(NamedKey::NumLock)),
            ],
        );
        check(
            "keypad",
            &[
                (Keysym::KP_Enter, Key::Named(NamedKey::Enter)),
                (Keysym::KP_Left, Key::Named(NamedKey::ArrowLeft)),
                (Keysym::KP_Delete, Key::Named(NamedKey::Delete)),
                (Keysym::KP_F4, Key::Named(NamedKey::F4)),
            ],
        );
        check(
            "function",
            &[
                (Keysym::F1, Key::Named(NamedKey::F1)),
                (Keysym::F12, Key::Named(NamedKey::F12)),
                (Keysym::F35, Key::Named(NamedKey::F35)),
            ],
        );
    }

    #[test]
    fn modifier_keys_and_super_is_super() {
        check(
            "modifiers",
            &[
                (Keysym::Shift_L, Key::Named(NamedKey::Shift)),
                (Keysym::Control_R, Key::Named(NamedKey::Control)),
                (Keysym::Alt_L, Key::Named(NamedKey::Alt)),
                (Keysym::Super_L, Key::Named(NamedKey::Meta)),
                (Keysym::Super_R, Key::Named(NamedKey::Meta)),
                (Keysym::ISO_Level3_Shift, Key::Named(NamedKey::AltGraph)),
                (Keysym::ISO_Left_Tab, Key::Named(NamedKey::Tab)),
                (Keysym::ISO_Next_Group, Key::Named(NamedKey::GroupNext)),
            ],
        );
    }

    #[test]
    fn terminal_media_and_vendor_keys() {
        check(
            "3270",
            &[
                (Keysym::_3270_EraseEOF, Key::Named(NamedKey::EraseEof)),
                (Keysym::_3270_Enter, Key::Named(NamedKey::Enter)),
            ],
        );
        check(
            "xf86",
            &[
                (
                    Keysym::XF86_MonBrightnessUp,
                    Key::Named(NamedKey::BrightnessUp),
                ),
                (
                    Keysym::XF86_AudioRaiseVolume,
                    Key::Named(NamedKey::AudioVolumeUp),
                ),
                (
                    Keysym::XF86_AudioMute,
                    Key::Named(NamedKey::AudioVolumeMute),
                ),
                (Keysym::XF86_AudioPlay, Key::Named(NamedKey::MediaPlay)),
                (
                    Keysym::XF86_Calculater,
                    Key::Named(NamedKey::LaunchApplication2),
                ),
                (Keysym::XF86_Sleep, Key::Named(NamedKey::Standby)),
            ],
        );
        check(
            "sun",
            &[
                (Keysym::SUN_Copy, Key::Named(NamedKey::Copy)),
                (
                    Keysym::SUN_VideoRaiseBrightness,
                    Key::Named(NamedKey::BrightnessUp),
                ),
            ],
        );
    }

    #[test]
    fn text_keys_fall_back_to_characters() {
        let cases: &[(&str, Keysym, Option<&str>, Key)] = &[
            ("letter", Keysym::a, Some("a"), Key::Character("a".into())),
            ("shifted", Keysym::A, Some("A"), Key::Character("A".into())),
            (
                "space",
                Keysym::space,
                Some(" "),
                Key::Character(" ".into()),
            ),
            (
                "ctrl+c is c",
                Keysym::c,
                Some("\u{3}"),
                Key::Character("c".into()),
            ),
            (
                "release has no text",
                Keysym::x,
                None,
                Key::Character("x".into()),
            ),
            (
                "named wins over text",
                Keysym::Return,
                Some("\r"),
                Key::Named(NamedKey::Enter),
            ),
            (
                "nothing",
                Keysym::NoSymbol,
                None,
                Key::Named(NamedKey::Unidentified),
            ),
        ];
        for (name, keysym, text, want) in cases {
            assert_eq!(logical_key(*keysym, *text), *want, "{name}");
        }
    }

    #[test]
    fn locations() {
        let cases = [
            (Keysym::Shift_L, Location::Left),
            (Keysym::Super_R, Location::Right),
            (Keysym::KP_5, Location::Numpad),
            (Keysym::KP_Enter, Location::Numpad),
            (Keysym::a, Location::Standard),
            (Keysym::Return, Location::Standard),
        ];
        for (keysym, want) in cases {
            assert_eq!(location(keysym), want, "{keysym:?}");
        }
    }
}
