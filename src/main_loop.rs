#![allow(clippy::while_float)]

use std::time::{Duration, Instant};

use glfw_sys::Key;

use crate::profiler::{mark_frame_end, profile};
use crate::window::{Resolution, Window};

pub struct MainLoop {
    window: Window,
    running: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Event {
    KeyPress(Key),
    KeyRelease(Key),
    WindowResize(u32, u32),
    MouseMove(f32, f32),
    MousePress(i32),
    MouseRelease(i32),
    MouseScroll(f32, f32),
}

impl MainLoop {
    pub fn new() -> Self {
        let window = Window::new(Resolution::Windowed(1024, 768), 0, "egui_glfw_mdi demo");

        Self { window, running: true }
    }

    pub fn run(mut self) {
        self.init();

        let update_rate = 64;
        let fps_limit = 500.;
        let dt = 1. / update_rate as f32;

        let mut t = 0.;
        let mut current = Instant::now();
        let mut accum = 0.;

        while self.running {
            let start = Instant::now();
            let elapsed = start - current;

            current = start;
            accum += elapsed.as_secs_f32();

            self.poll_events();

            while accum >= dt {
                self.update(t, dt);
                t += dt;
                accum -= dt;
            }

            self.render(accum / dt);

            limit_fps(fps_limit, &start);
            mark_frame_end();
        }
    }

    fn init(&mut self) {
        let ptr = self as *mut Self;

        self.window.set_event_dest(ptr);
        self.window.set_viewport();
    }

    fn poll_events(&mut self) {
        profile!();
        self.window.poll_events();

        if self.window.should_close() {
            self.running = false;
        }
    }

    fn update(&mut self, t: f32, dt: f32) {
        profile!();
    }

    fn render(&mut self, alpha: f32) {
        profile!();
        self.swap_buffers();
    }

    fn swap_buffers(&self) {
        profile!();
        self.window.swap_buffers();
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::KeyPress(Key::Escape) => self.running = false,
            Event::WindowResize(..) => self.window.set_viewport(),
            _ => {}
        }
    }

    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }
}

fn limit_fps(target_fps: f32, start: &Instant) {
    profile!();
    let frame_time = start.elapsed();
    let target_frame_time = Duration::from_secs_f32(1. / target_fps);

    if let Some(to_sleep) = target_frame_time.checked_sub(frame_time) {
        std::thread::sleep(to_sleep);
    }
}
