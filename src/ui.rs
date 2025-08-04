use std::ptr;

use egui::ahash::HashMap;
use egui::epaint::{ImageDelta, Primitive};
use egui::load::SizedTexture;
use egui::{Context, Pos2, RawInput, Rect, TextureId, Vec2};

use crate::gl::{Buffer, Program, Shader, TextureArray, VertexArray, include_shader};
use crate::main_loop::Event;
use crate::profiler::profile;
use crate::utils::CheckError;
use crate::window::Window;

pub struct UI {
    prog: Program,
    vao: VertexArray,
    vertices: Buffer,
    elements: Buffer,
    commands: Buffer,
    ctx: Context,
    input: RawInput,
    mouse_pos: Pos2,

    pub textures: TexturePool,
}

pub struct TexturePool {
    array: TextureArray,
    infos: HashMap<TextureId, TextureInfo>,
    max_width: usize,
    max_height: usize,
    max_depth: i32,
    next_layer: i32,
}

#[derive(Clone, Copy)]
struct TextureInfo {
    layer: i32,
    width: i32,
    height: i32,
}

#[repr(C, packed)]
struct DrawElementsCmd {
    count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    texture_layer: u32, // was base_instance
    uv_scale_x: f32,
    uv_scale_y: f32,
    scissor_x: f32,
    scissor_y: f32,
    scissor_w: f32,
    scissor_h: f32,
}

impl UI {
    pub fn new(window: &Window, max_texture_width: usize, max_texture_height: usize) -> Self {
        let vs = Shader::new(gl::VERTEX_SHADER, include_shader!("ui.vert"));
        let fs = Shader::new(gl::FRAGMENT_SHADER, include_shader!("ui.frag"));
        let prog = Program::new([vs, fs], ["screenSize", "texArray", "texLayer", "uvScale"]);

        let vao = VertexArray::new();
        let vertices = Buffer::new(gl::ARRAY_BUFFER);
        let elements = Buffer::new(gl::ELEMENT_ARRAY_BUFFER);
        let commands = Buffer::new(gl::DRAW_INDIRECT_BUFFER);

        let ctx = Context::default();
        let input = initial_input(window);
        let mouse_pos = Pos2::new(0., 0.);
        let textures = TexturePool::new(max_texture_width, max_texture_height);

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

        Self { prog, vao, vertices, elements, commands, ctx, input, mouse_pos, textures }
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
        self.render_mdi(ui);
    }

    fn render_mdi(&mut self, ui: impl FnMut(&Context)) {
        profile!();
        let output = self.ctx.run(self.input.clone(), ui);

        self.prog.enable();
        self.vao.enable();
        self.textures.array.enable();

        // There's probably a better way to do this: instead of binding draw commands as SSBO and
        // accessing them via gl_DrawID (requires GL 4.6), bind them as GL_ARRAY_BUFFER and access
        // via attributes and attribute divisors. Or just make a separate buffer for texture infos.
        self.commands.set_ssbo_binding(0);

        for (id, delta) in output.textures_delta.set {
            self.update_texture(id, &delta);
        }

        let clip_primitives = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        let command_count = self.upload_to_buffers(clip_primitives);
        let stride = size_of::<DrawElementsCmd>() as i32;

        unsafe {
            gl::Disable(gl::CULL_FACE);
            gl::Disable(gl::DEPTH_TEST);

            gl::MultiDrawElementsIndirect(
                gl::TRIANGLES,
                gl::UNSIGNED_INT,
                ptr::null(),
                command_count,
                stride,
            );

            gl::Enable(gl::CULL_FACE);
            gl::Enable(gl::DEPTH_TEST);
        }

        self.input.events.clear();
    }

    fn upload_to_buffers(&self, clip_primitives: Vec<egui::ClippedPrimitive>) -> i32 {
        let (width, height) = self.window_size();

        let mut vertices = vec![];
        let mut elements = vec![];
        let mut commands = vec![];

        for clip_primitive in clip_primitives {
            if let Primitive::Mesh(mesh) = clip_primitive.primitive {
                let Some(info) = self.textures.fetch(mesh.texture_id) else {
                    println!("warning: unknown texture ID {:?}", mesh.texture_id);
                    continue;
                };

                let rect = clip_primitive.clip_rect;
                let clip_min_x = rect.min.x.round().clamp(0., width);
                let clip_min_y = rect.min.y.round().clamp(0., height);
                let clip_max_x = rect.max.x.round().clamp(clip_min_x, width);
                let clip_max_y = rect.max.y.round().clamp(clip_min_y, height);

                let command = DrawElementsCmd {
                    count: mesh.indices.len() as u32,
                    instance_count: 1,
                    first_index: elements.len() as u32,
                    base_vertex: vertices.len() as i32,
                    texture_layer: info.layer as u32,
                    uv_scale_x: info.width as f32 / self.textures.max_width as f32,
                    uv_scale_y: info.height as f32 / self.textures.max_height as f32,
                    scissor_x: clip_min_x,
                    scissor_y: height - clip_max_y,
                    scissor_w: clip_max_x - clip_min_x,
                    scissor_h: clip_max_y - clip_min_y,
                };

                vertices.extend(mesh.vertices);
                elements.extend(mesh.indices);
                commands.push(command);
            }
        }

        self.vertices.enable();
        self.elements.enable();
        self.commands.enable();

        self.vertices.upload_data(&vertices, gl::STREAM_DRAW);
        self.elements.upload_data(&elements, gl::STREAM_DRAW);
        self.commands.upload_data(&commands, gl::STREAM_DRAW);

        commands.len() as i32
    }

    #[allow(unused)]
    fn render_simple(&mut self, ui: impl FnMut(&Context)) {
        profile!();
        let output = self.ctx.run(self.input.clone(), ui);

        self.textures.array.enable();

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
        let egui::ImageData::Color(image) = &delta.image;
        let [w, h] = image.size;
        let [x, y] = delta.pos.unwrap_or([0, 0]);
        let info = self.textures.fetch_or_add(id, w, h);

        if image.pixels.len() != w * h {
            println!("warning: UI texture len mismatch: {} != {w} * {h}", image.pixels.len());
        }

        self.textures.array.upload(x as i32, y as i32, info.layer, w, h, gl::RGBA, &image.pixels);
        self.textures.array.generate_mipmaps();
    }

    fn render_mesh(&self, mesh: &egui::Mesh) {
        let Some(info) = self.textures.fetch(mesh.texture_id) else {
            println!("warning: unknown texture ID {:?}", mesh.texture_id);
            return;
        };

        let scale_x = info.width as f32 / self.textures.max_width as f32;
        let scale_y = info.height as f32 / self.textures.max_height as f32;
        let count = mesh.indices.len() as i32;

        self.prog.set_uniform_1i(2, info.layer);
        self.prog.set_uniform_2f(3, scale_x, scale_y);

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
            Event::MouseScroll(x, y) => {
                self.input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: Vec2::new(*x, *y),
                    modifiers: egui::Modifiers::default(),
                });
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
            modifiers: egui::Modifiers::default(),
        };

        self.input.events.push(event);
    }
}

impl TexturePool {
    fn new(max_width: usize, max_height: usize) -> Self {
        // this equation comes from glTexStorage3D reference page
        let max_depth = i32::max(max_width as i32, max_height as i32).ilog2() as i32 + 1;

        let array = TextureArray::new(gl::RGBA8, max_width as i32, max_height as i32, max_depth);
        let infos = HashMap::default();
        let next_layer = 0;

        Self { array, infos, max_width, max_height, max_depth, next_layer }
    }

    pub fn missing(&mut self, size: usize, cell_size_exp: usize) -> SizedTexture {
        let cell_size = 1 << cell_size_exp;
        let col_a = 0xff_00_00_00;
        let col_b = 0xff_ff_00_ff;

        let mut pixels = vec![0_u32; size * size];

        for y in 0..size {
            for x in 0..size {
                let col = if (x & cell_size) ^ (y & cell_size) == 0 { col_b } else { col_a };

                pixels[y * size + x] = col;
            }
        }

        self.insert(size, size, &pixels)
    }

    pub fn xor(&mut self) -> SizedTexture {
        let size = 256;
        let mut pixels = vec![0_u32; size * size];

        for y in 0..size {
            for x in 0..size {
                let byte = (y as u32) ^ (x as u32);
                let rgb = (255 << 24) | (byte << 16) | (byte << 8) | (byte);

                pixels[y * size + x] = rgb;
            }
        }

        self.insert(size, size, &pixels)
    }

    pub fn rgb_slice(&mut self) -> SizedTexture {
        let size = 256;
        let mut pixels = vec![0_u32; size * size];

        for y in 0..size {
            for x in 0..size {
                let r = x as u32;
                let g = y as u32;
                let b = 128;
                let rgb = (255 << 24) | (b << 16) | (g << 8) | r;

                pixels[y * size + x] = rgb;
            }
        }

        self.insert(size, size, &pixels)
    }

    fn insert<T>(&mut self, w: usize, h: usize, pixels: &[T]) -> SizedTexture {
        assert!(w <= self.max_width && h <= self.max_height);
        assert!(self.next_layer < self.max_depth);

        let id = TextureId::User(self.next_layer as u64);
        let size = Vec2::new(w as f32, h as f32);

        self.array.enable();
        self.array.upload(0, 0, self.next_layer, w, h, gl::RGBA, pixels);
        self.infos.insert(id, TextureInfo::new(self.next_layer, w as i32, h as i32));

        self.next_layer += 1;

        SizedTexture::new(id, size)
    }

    fn fetch_or_add(&mut self, id: TextureId, w: usize, h: usize) -> TextureInfo {
        *self.infos.entry(id).or_insert_with(|| {
            let info = TextureInfo::new(self.next_layer, w as i32, h as i32);

            self.next_layer += 1;

            info
        })
    }

    fn fetch(&self, id: TextureId) -> Option<&TextureInfo> {
        self.infos.get(&id)
    }
}

impl TextureInfo {
    fn new(layer: i32, width: i32, height: i32) -> Self {
        Self { layer, width, height }
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
