//! Window, input and presentation via winit + softbuffer.
//!
//! softbuffer hands frames to the compositor through shared memory (wl_shm on
//! Wayland, MIT-SHM on X11) instead of copying every frame through the X11
//! socket, and winit runs natively on Wayland (no XWayland detour). The API
//! mimics the small part of minifb the frontend used, so key codes stay
//! `minifb::Key`.
use minifb::Key;
use std::collections::HashSet;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key as WKey, KeyCode, PhysicalKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Fullscreen, Window as WWindow, WindowId};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyRepeat {
    Yes,
    No,
}

#[derive(Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

struct State {
    title: String,
    init: (u32, u32),
    window: Option<Rc<WWindow>>,
    ctx: Option<softbuffer::Context<Rc<WWindow>>>,
    surface: Option<softbuffer::Surface<Rc<WWindow>, Rc<WWindow>>>,
    open: bool,
    focused: bool,
    down: HashSet<Key>,
    pressed: Vec<(Key, bool)>,
    mouse: Option<(f32, f32)>,
    buttons: [bool; 3],
    wheel: f32,
    size: (u32, u32),
}

const LETTERS: [Key; 26] = [
    Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K, Key::L, Key::M,
    Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X, Key::Y, Key::Z,
];

fn map_code(c: KeyCode) -> Option<Key> {
    use KeyCode as K;
    Some(match c {
        K::Escape => Key::Escape,
        K::F1 => Key::F1,
        K::F2 => Key::F2,
        K::F3 => Key::F3,
        K::F4 => Key::F4,
        K::F5 => Key::F5,
        K::F6 => Key::F6,
        K::F7 => Key::F7,
        K::F8 => Key::F8,
        K::F9 => Key::F9,
        K::F10 => Key::F10,
        K::F11 => Key::F11,
        K::F12 => Key::F12,
        K::Space => Key::Space,
        K::Tab => Key::Tab,
        K::Enter => Key::Enter,
        K::NumpadEnter => Key::NumPadEnter,
        K::Backspace => Key::Backspace,
        K::ShiftLeft => Key::LeftShift,
        K::ShiftRight => Key::RightShift,
        K::ControlLeft => Key::LeftCtrl,
        K::ControlRight => Key::RightCtrl,
        K::AltLeft => Key::LeftAlt,
        K::AltRight => Key::RightAlt,
        K::Insert => Key::Insert,
        K::Delete => Key::Delete,
        K::Home => Key::Home,
        K::End => Key::End,
        K::PageUp => Key::PageUp,
        K::PageDown => Key::PageDown,
        K::ArrowUp => Key::Up,
        K::ArrowDown => Key::Down,
        K::ArrowLeft => Key::Left,
        K::ArrowRight => Key::Right,
        K::Numpad0 => Key::NumPad0,
        K::Numpad1 => Key::NumPad1,
        K::Numpad2 => Key::NumPad2,
        K::Numpad3 => Key::NumPad3,
        K::Numpad4 => Key::NumPad4,
        K::Numpad5 => Key::NumPad5,
        K::Numpad6 => Key::NumPad6,
        K::Numpad7 => Key::NumPad7,
        K::Numpad8 => Key::NumPad8,
        K::Numpad9 => Key::NumPad9,
        K::NumpadAdd => Key::NumPadPlus,
        K::NumpadSubtract => Key::NumPadMinus,
        K::Comma => Key::Comma,
        K::Period => Key::Period,
        K::Minus => Key::Minus,
        K::Semicolon => Key::Semicolon,
        K::Quote => Key::Apostrophe,
        K::Slash => Key::Slash,
        K::Backslash => Key::Backslash,
        K::BracketLeft => Key::LeftBracket,
        K::BracketRight => Key::RightBracket,
        K::Digit0 => Key::Key0,
        K::Digit1 => Key::Key1,
        K::Digit2 => Key::Key2,
        K::Digit3 => Key::Key3,
        K::Digit4 => Key::Key4,
        K::Digit5 => Key::Key5,
        K::Digit6 => Key::Key6,
        K::Digit7 => Key::Key7,
        K::Digit8 => Key::Key8,
        K::Digit9 => Key::Key9,
        K::KeyA => Key::A,
        K::KeyB => Key::B,
        K::KeyC => Key::C,
        K::KeyD => Key::D,
        K::KeyE => Key::E,
        K::KeyF => Key::F,
        K::KeyG => Key::G,
        K::KeyH => Key::H,
        K::KeyI => Key::I,
        K::KeyJ => Key::J,
        K::KeyK => Key::K,
        K::KeyL => Key::L,
        K::KeyM => Key::M,
        K::KeyN => Key::N,
        K::KeyO => Key::O,
        K::KeyP => Key::P,
        K::KeyQ => Key::Q,
        K::KeyR => Key::R,
        K::KeyS => Key::S,
        K::KeyT => Key::T,
        K::KeyU => Key::U,
        K::KeyV => Key::V,
        K::KeyW => Key::W,
        K::KeyX => Key::X,
        K::KeyY => Key::Y,
        K::KeyZ => Key::Z,
        _ => return None,
    })
}

/// Letters follow the keyboard layout (QWERTZ: the key labelled Z is Z);
/// everything else uses the physical position.
fn map_key(ev: &KeyEvent) -> Option<Key> {
    if let PhysicalKey::Code(code) = ev.physical_key {
        let is_letter = format!("{:?}", code).starts_with("Key");
        if is_letter {
            if let WKey::Character(s) = &ev.key_without_modifiers() {
                if let Some(c) = s.chars().next() {
                    let c = c.to_ascii_uppercase();
                    if c.is_ascii_uppercase() {
                        return Some(LETTERS[(c as u8 - b'A') as usize]);
                    }
                }
            }
        }
        return map_code(code);
    }
    None
}

use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

impl ApplicationHandler for State {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WWindow::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(PhysicalSize::new(self.init.0, self.init.1));
        let w = Rc::new(el.create_window(attrs).expect("cannot open window"));
        let ctx = softbuffer::Context::new(w.clone()).expect("softbuffer context");
        let surface = softbuffer::Surface::new(&ctx, w.clone()).expect("softbuffer surface");
        let s = w.inner_size();
        self.size = (s.width.max(1), s.height.max(1));
        self.window = Some(w);
        self.ctx = Some(ctx);
        self.surface = Some(surface);
        self.focused = true;
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.open = false,
            WindowEvent::Resized(s) => self.size = (s.width.max(1), s.height.max(1)),
            WindowEvent::Focused(f) => {
                self.focused = f;
                if !f {
                    self.down.clear();
                    self.buttons = [false; 3];
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(k) = map_key(&event) {
                    match event.state {
                        ElementState::Pressed => {
                            self.pressed.push((k, event.repeat));
                            self.down.insert(k);
                        }
                        ElementState::Released => {
                            self.down.remove(&k);
                            // letters may change mapping with modifiers: release all variants
                            if let PhysicalKey::Code(c) = event.physical_key {
                                if let Some(pk) = map_code(c) {
                                    self.down.remove(&pk);
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => self.mouse = Some((position.x as f32, position.y as f32)),
            WindowEvent::CursorLeft { .. } => self.mouse = None,
            WindowEvent::MouseInput { state, button, .. } => {
                let i = match button {
                    winit::event::MouseButton::Left => 0,
                    winit::event::MouseButton::Right => 1,
                    winit::event::MouseButton::Middle => 2,
                    _ => return,
                };
                self.buttons[i] = state == ElementState::Pressed;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.wheel += match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => (y / 40.0) as f32,
                }
            }
            _ => {}
        }
    }
}

pub struct Win {
    el: EventLoop<()>,
    st: State,
    frame_pressed: Vec<(Key, bool)>,
    frame_wheel: f32,
    clicks: [bool; 3],
}

impl Win {
    pub fn new(title: &str, w: usize, h: usize) -> Win {
        let mut el = EventLoop::new().expect("no display available");
        let mut st = State {
            title: title.to_string(),
            init: (w as u32, h as u32),
            window: None,
            ctx: None,
            surface: None,
            open: true,
            focused: true,
            down: HashSet::new(),
            pressed: Vec::new(),
            mouse: None,
            buttons: [false; 3],
            wheel: 0.0,
            size: (w as u32, h as u32),
        };
        for _ in 0..200 {
            let _ = el.pump_app_events(Some(Duration::from_millis(5)), &mut st);
            if st.window.is_some() {
                break;
            }
        }
        if st.window.is_none() {
            panic!("cannot open window");
        }
        Win { el, st, frame_pressed: Vec::new(), frame_wheel: 0.0, clicks: [false; 3] }
    }

    /// Processes pending window events; call once per loop iteration.
    pub fn pump(&mut self) {
        let before = self.st.buttons;
        if let PumpStatus::Exit(_) = self.el.pump_app_events(Some(Duration::ZERO), &mut self.st) {
            self.st.open = false;
        }
        self.frame_pressed = std::mem::take(&mut self.st.pressed);
        self.frame_wheel = std::mem::take(&mut self.st.wheel);
        // a click shorter than one frame still counts
        for i in 0..3 {
            self.clicks[i] = self.st.buttons[i] && !before[i];
        }
    }

    pub fn is_open(&self) -> bool {
        self.st.open
    }
    pub fn is_active(&self) -> bool {
        self.st.focused
    }
    pub fn get_keys_pressed(&self, r: KeyRepeat) -> Vec<Key> {
        self.frame_pressed.iter().filter(|(_, rep)| r == KeyRepeat::Yes || !rep).map(|(k, _)| *k).collect()
    }
    pub fn is_key_down(&self, k: Key) -> bool {
        self.st.down.contains(&k)
    }
    pub fn get_mouse_down(&self, b: MouseButton) -> bool {
        let i = b as usize;
        self.st.buttons[i] || self.clicks[i]
    }
    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        (self.frame_wheel != 0.0).then_some((0.0, self.frame_wheel))
    }
    pub fn mouse_pos(&self) -> Option<(f32, f32)> {
        self.st.mouse
    }
    pub fn get_size(&self) -> (usize, usize) {
        (self.st.size.0 as usize, self.st.size.1 as usize)
    }
    fn window(&self) -> &WWindow {
        self.st.window.as_ref().unwrap()
    }
    pub fn set_cursor_visibility(&self, v: bool) {
        self.window().set_cursor_visible(v);
    }
    pub fn set_fullscreen(&self, on: bool) -> bool {
        self.window().set_fullscreen(if on { Some(Fullscreen::Borderless(None)) } else { None });
        true
    }
    pub fn set_size(&self, w: usize, h: usize) {
        let _ = self.window().request_inner_size(PhysicalSize::new(w as u32, h as u32));
    }

    /// Presents a buffer of exactly the current window size.
    pub fn update_with_buffer(&mut self, buf: &[u32], w: usize, h: usize) {
        let (Some(nw), Some(nh)) = (NonZeroU32::new(w as u32), NonZeroU32::new(h as u32)) else { return };
        let Some(surface) = self.st.surface.as_mut() else { return };
        if surface.resize(nw, nh).is_err() {
            return;
        }
        if let Ok(mut b) = surface.buffer_mut() {
            if b.len() == buf.len() {
                b.copy_from_slice(buf);
                let _ = b.present();
            }
        }
    }
}
