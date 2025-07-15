#![allow(clippy::while_float)]

use std::time::{Duration, Instant};

use glfw_sys::Key;

use crate::gl::{Texture, init_gl};
use crate::profiler::{mark_frame_end, profile};
use crate::ui::UI;
use crate::window::{Resolution, Window};

pub struct MainLoop {
    ui: UI,
    textures: Vec<Texture>,
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
        let window = Window::new(Resolution::Windowed(1024, 768), 0, "egui_glfw_mdi");
        let ui = UI::new(&window);
        let textures = vec![Texture::missing(64, 3), Texture::xor(), Texture::rgb_slice()];
        let running = true;

        Self { ui, textures, window, running }
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
            self.swap_buffers();

            limit_fps(fps_limit, &start);
            mark_frame_end();
        }
    }

    fn init(&mut self) {
        let ptr = self as *mut Self;

        self.window.set_event_dest(ptr);
        self.window.set_viewport();

        init_gl();
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
        self.ui.update(t, dt);
    }

    fn render(&mut self, _alpha: f32) {
        profile!();

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        let grid_size_y = 29;
        let grid_size_x = 20;

        self.ui.render(|ctx| {
            egui::Window::new("hi").show(ctx, |ui| {
                ui.label("some images");

                ui.horizontal(|ui| {
                    for texture in &self.textures {
                        ui.image(texture.to_img_source(64., 64.));
                    }
                });

                egui::Grid::new("labels").show(ui, |ui| {
                    for y in 0..grid_size_y {
                        for x in 0..grid_size_x {
                            ui.label(format!("{y},{x}"));
                        }

                        ui.end_row();
                    }
                });
            });
        });
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

        self.ui.handle_event(&event);
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
