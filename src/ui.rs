use std::ptr;

use egui::ahash::HashMap;
use egui::epaint::{ImageDelta, Primitive};
use egui::{Context, ImageData, Mesh, Modifiers, Pos2, RawInput, Rect, TextureId, Vec2};

use crate::gl::{Buffer, Program, Shader, Texture, VertexArray, include_shader};
use crate::main_loop::Event;
use crate::profiler::profile;
use crate::utils::CheckError;
use crate::window::Window;

pub struct UI {
    prog: Program,
    vao: VertexArray,
    vertices: Buffer,
    elements: Buffer,
    ctx: Context,
    input: RawInput,
    mouse_pos: Pos2,
    textures: HashMap<TextureId, Texture>,
}

impl UI {
    pub fn new(window: &Window) -> Self {
        let vs = Shader::new(gl::VERTEX_SHADER, include_shader!("ui.vert"));
        let fs = Shader::new(gl::FRAGMENT_SHADER, include_shader!("ui.frag"));
        let prog = Program::new([vs, fs], ["screenSize", "tex"]);

        let vao = VertexArray::new();
        let vertices = Buffer::new(gl::ARRAY_BUFFER);
        let elements = Buffer::new(gl::ELEMENT_ARRAY_BUFFER);

        let ctx = Context::default();
        let input = initial_input(window);
        let mouse_pos = Pos2::new(0., 0.);
        let textures = initial_textures();

        let (w, h) = window.size();

        vao.enable();
        vertices.enable();

        let size = 2 * 4 + 2 * 4 + 4 * 1;
        vao.def_attr(0, 2, gl::FLOAT, size, 0);
        vao.def_attr(1, 2, gl::FLOAT, size, 2 * 4);
        vao.def_attr(2, 4, gl::UNSIGNED_BYTE, size, 4 * 4);

        prog.enable();
        prog.set_uniform_2f(0, w as f32, h as f32);
        prog.set_uniform_1i(1, 0);

        ctx.tessellation_options_mut(|opt| opt.feathering = false);

        Self { prog, vao, vertices, elements, ctx, input, mouse_pos, textures }
    }

    fn window_size(&self) -> (f32, f32) {
        let max = self.input.screen_rect.or_err("screen_rect unset").max;

        (max.x, max.y)
    }

    pub fn update(&mut self, t: f32, dt: f32) {
        self.input.time = Some(t.into());
        self.input.predicted_dt = dt;
    }

    pub fn render(&mut self, ui: impl FnMut(&Context)) {
        self.render_simple(ui);
    }

    fn render_simple(&mut self, ui: impl FnMut(&Context)) {
        profile!();
        let output = self.ctx.run(self.input.clone(), ui);

        for (id, delta) in output.textures_delta.set {
            self.update_texture(id, &delta);
        }

        let (width, height) = self.window_size();
        let clip_primitives = self.ctx.tessellate(output.shapes, output.pixels_per_point);

        self.prog.enable();

        self.vao.enable();
        self.vertices.enable();
        self.elements.enable();

        unsafe {
            gl::Disable(gl::CULL_FACE);
            gl::Disable(gl::DEPTH_TEST);
        }

        for clip_primitive in clip_primitives {
            set_clip_rect(clip_primitive.clip_rect, width, height);

            if let Primitive::Mesh(mesh) = clip_primitive.primitive {
                self.render_mesh(&mesh);
            }
        }

        unsafe {
            gl::Enable(gl::CULL_FACE);
            gl::Enable(gl::DEPTH_TEST);
        }

        self.input.events.clear();
    }

    fn update_texture(&mut self, id: TextureId, delta: &ImageDelta) {
        let texture = self.textures.entry(id).or_insert_with(Texture::new);

        texture.enable();

        let ImageData::Color(image) = &delta.image;
        let [w, h] = image.size;

        if image.pixels.len() != w * h {
            println!("warning: UI texture len mismatch: {} != {w} * {h}", image.pixels.len());
        }

        match delta.pos {
            Some([x, y]) => texture.upload_subdata(x, y, w, h, gl::RGBA, &image.pixels),
            None => texture.upload_data(gl::RGBA8, w, h, gl::RGBA, &image.pixels),
        }

        texture.generate_mipmaps();
    }

    fn render_mesh(&self, mesh: &Mesh) {
        match mesh.texture_id {
            TextureId::Managed(_) => {
                let Some(texture) = self.textures.get(&mesh.texture_id) else {
                    println!("unknown managed UI texture: {:?}", mesh.texture_id);
                    return;
                };
                texture.enable();
            }
            TextureId::User(id) => unsafe {
                gl::BindTexture(gl::TEXTURE_2D, id as u32);
            },
        }

        let count = mesh.indices.len() as i32;

        self.vertices.upload_data(&mesh.vertices, gl::STREAM_DRAW);
        self.elements.upload_data(&mesh.indices, gl::STREAM_DRAW);

        unsafe {
            gl::DrawElements(gl::TRIANGLES, count, gl::UNSIGNED_INT, ptr::null());
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::WindowResize(w, h) => {
                self.input.screen_rect = screen_rect(*w, *h);
                self.prog.enable();
                self.prog.set_uniform_2f(0, *w as f32, *h as f32);
            }
            Event::MouseMove(x, y) => {
                self.mouse_pos.x = *x;
                self.mouse_pos.y = *y;
                self.input.events.push(egui::Event::PointerMoved(self.mouse_pos));
            }
            Event::MousePress(btn) => self.mouse_press_event(*btn, true),
            Event::MouseRelease(btn) => self.mouse_press_event(*btn, false),
            _ => {}
        }
    }

    fn mouse_press_event(&mut self, raw: i32, pressed: bool) {
        let event = egui::Event::PointerButton {
            pos: self.mouse_pos,
            button: egui_mouse_button(raw),
            pressed,
            modifiers: Modifiers::default(),
        };

        self.input.events.push(event);
    }
}

fn initial_input(window: &Window) -> RawInput {
    let (width, height) = window.size();
    let mut max_texture_size = 0;

    unsafe {
        gl::GetIntegerv(gl::MAX_TEXTURE_SIZE, &mut max_texture_size);
    }

    RawInput {
        screen_rect: screen_rect(width, height),
        max_texture_side: Some(max_texture_size as usize),
        time: Some(0.),
        ..Default::default()
    }
}

fn screen_rect(w: u32, h: u32) -> Option<Rect> {
    let min = Pos2::new(0., 0.);
    let size = Vec2::new(w as f32, h as f32);
    let rect = Rect::from_min_size(min, size);

    Some(rect)
}

fn initial_textures() -> HashMap<TextureId, Texture> {
    let id = TextureId::Managed(0);
    let texture = Texture::new();
    let mut out = HashMap::default();

    texture.enable();
    texture.upload_data(gl::RGBA8, 1, 1, gl::RGBA, &[0_u8, 0, 0, 0]);

    out.entry(id).or_insert(texture);

    out
}

fn set_clip_rect(rect: Rect, width: f32, height: f32) {
    let clip_min_x = (rect.min.x.round() as i32).clamp(0, width as i32);
    let clip_min_y = (rect.min.y.round() as i32).clamp(0, height as i32);
    let clip_max_x = (rect.max.x.round() as i32).clamp(clip_min_x, width as i32);
    let clip_max_y = (rect.max.y.round() as i32).clamp(clip_min_y, height as i32);

    unsafe {
        gl::Scissor(
            clip_min_x,
            height as i32 - clip_max_y,
            clip_max_x - clip_min_x,
            clip_max_y - clip_min_y,
        );
    }
}

fn egui_mouse_button(raw: i32) -> egui::PointerButton {
    match raw {
        2 => egui::PointerButton::Secondary,
        3 => egui::PointerButton::Middle,
        4 => egui::PointerButton::Extra1,
        5 => egui::PointerButton::Extra2,
        _ => egui::PointerButton::Primary,
    }
}
