use std::ffi::c_char;

use crate::utils::{to_cstring, to_i32, to_isize, to_usize};

pub struct Shader {
    id: u32,
}

pub struct Program {
    id: u32,
    uniforms: Vec<i32>,
}

pub struct VertexArray {
    id: u32,
}

pub struct Buffer {
    ty: u32,
    id: u32,
}

pub struct Texture {
    id: u32,
}

macro_rules! include_shader {
    ($name: literal) => {
        include_str!(concat!("../shaders/", $name))
    };
}

pub(crate) use include_shader;

macro_rules! get_uniform_location {
    ($uniforms: expr, $idx: expr) => {
        match $uniforms.get($idx) {
            Some(v) => *v,
            None => {
                println!("warning: uniform idx {} not found", $idx);
                return;
            }
        }
    };
}

impl Shader {
    pub fn new(ty: u32, src: &str) -> Self {
        let ptr = src.as_ptr().cast();
        let len = to_i32(src.len());
        let id;

        unsafe {
            id = gl::CreateShader(ty);
            gl::ShaderSource(id, 1, &ptr, &len);
            gl::CompileShader(id);
        }

        check_compile_status(id, ty);

        Self { id }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

impl Program {
    pub fn new<S, U>(shaders: S, uniform_names: U) -> Self
    where
        S: IntoIterator<Item = Shader>,
        U: IntoIterator<Item = &'static str>,
    {
        let id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe {
                gl::AttachShader(id, shader.id);
            }
        }

        unsafe {
            gl::LinkProgram(id);
        }

        check_link_status(id);

        let mut uniforms = Vec::with_capacity(8);

        for name in uniform_names {
            let cstr = to_cstring(name);
            let loc = unsafe { gl::GetUniformLocation(id, cstr.as_ptr()) };

            uniforms.push(loc);
        }

        Self { id, uniforms }
    }

    pub fn enable(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn set_uniform_1i(&self, idx: usize, value: i32) {
        let location = get_uniform_location!(self.uniforms, idx);

        unsafe {
            gl::Uniform1i(location, value);
        }
    }

    pub fn set_uniform_2f(&self, idx: usize, a: f32, b: f32) {
        let location = get_uniform_location!(self.uniforms, idx);

        unsafe {
            gl::Uniform2f(location, a, b);
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

impl VertexArray {
    pub fn new() -> Self {
        let mut id = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut id);
        }

        Self { id }
    }

    pub fn enable(&self) {
        unsafe {
            gl::BindVertexArray(self.id);
        }
    }

    pub fn def_attr(&self, idx: u32, size: i32, ty: u32, stride: usize, offset: usize) {
        unsafe {
            gl::VertexAttribPointer(idx, size, ty, gl::FALSE, to_i32(stride), offset as *const _);
            gl::EnableVertexAttribArray(idx);
        }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.id);
        }
    }
}

impl Buffer {
    pub fn new(ty: u32) -> Self {
        let mut id = 0;

        unsafe {
            gl::GenBuffers(1, &mut id);
        }

        Self { ty, id }
    }

    pub fn enable(&self) {
        unsafe {
            gl::BindBuffer(self.ty, self.id);
        }
    }

    pub fn upload_data<T>(&self, data: &[T], usage: u32) {
        let size = to_isize(size_of_val(data));

        unsafe {
            gl::BufferData(self.ty, size, data.as_ptr().cast(), usage);
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.id);
        }
    }
}

impl Texture {
    pub fn new() -> Self {
        let mut id = 0;

        unsafe {
            gl::GenTextures(1, &mut id);
        }

        Self { id }
    }

    pub fn missing(size: usize, cell_size_exp: usize) -> Self {
        let cell_size = 1 << cell_size_exp;
        let col_a = 0xff_00_00_00;
        let col_b = 0xff_ff_00_ff;

        let out = Self::new();
        let mut pattern = vec![0_u32; size * size];

        for y in 0..size {
            for x in 0..size {
                let col = if (x & cell_size) ^ (y & cell_size) == 0 { col_b } else { col_a };

                pattern[y * size + x] = col;
            }
        }

        out.enable();
        out.upload_data(gl::RGBA8, size, size, gl::RGBA, &pattern);
        out.generate_mipmaps();

        out
    }

    pub fn xor() -> Self {
        let size = 256;
        let out = Self::new();
        let mut pixels = vec![0_u32; size * size];

        for y in 0..size {
            for x in 0..size {
                let byte = (y as u32) ^ (x as u32);
                let rgb = (255 << 24) | (byte << 16) | (byte << 8) | (byte);

                pixels[y * size + x] = rgb;
            }
        }

        out.enable();
        out.upload_data(gl::RGBA8, size, size, gl::RGBA, &pixels);
        out.generate_mipmaps();

        out
    }

    pub fn rgb_slice() -> Self {
        let size = 256;
        let out = Self::new();
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

        out.enable();
        out.upload_data(gl::RGBA8, size, size, gl::RGBA, &pixels);
        out.generate_mipmaps();

        out
    }

    pub fn enable(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.id);
        }
    }

    pub fn upload_data<T>(&self, internal_format: u32, w: usize, h: usize, fmt: u32, data: &[T]) {
        let internal_format = internal_format as i32;
        let w = w as i32;
        let h = h as i32;
        let ty = gl::UNSIGNED_BYTE;
        let pixels = data.as_ptr().cast();

        unsafe {
            gl::TexImage2D(gl::TEXTURE_2D, 0, internal_format, w, h, 0, fmt, ty, pixels);
        }
    }

    pub fn upload_subdata<T>(&self, x: usize, y: usize, w: usize, h: usize, fmt: u32, data: &[T]) {
        let x = x as i32;
        let y = y as i32;
        let w = w as i32;
        let h = h as i32;
        let ty = gl::UNSIGNED_BYTE;
        let pixels = data.as_ptr().cast();

        unsafe {
            gl::TexSubImage2D(gl::TEXTURE_2D, 0, x, y, w, h, fmt, ty, pixels);
        }
    }

    pub fn generate_mipmaps(&self) {
        unsafe {
            gl::GenerateMipmap(gl::TEXTURE_2D);
        }
    }

    pub fn to_img_source(&self, w: f32, h: f32) -> egui::ImageSource {
        let usr_texture = egui::TextureId::User(self.id.into());
        let size = egui::Vec2::new(w, h);
        let sized_texture = egui::load::SizedTexture::new(usr_texture, size);

        egui::ImageSource::Texture(sized_texture)
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }
    }
}

fn check_compile_status(shader: u32, ty: u32) {
    unsafe {
        let mut success = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);

        if success == 1 {
            return;
        }

        let mut buf = vec![0; 512];
        let mut len = 0;
        gl::GetShaderInfoLog(shader, to_i32(buf.len()), &mut len, buf.as_mut_ptr());

        let tystr = shader_type_str(ty);
        let log = c_char_buf_to_string(&buf, len);

        panic!("failed to compile {tystr} shader:\n{log}");
    }
}

fn shader_type_str(ty: u32) -> String {
    match ty {
        gl::VERTEX_SHADER => "vertex".to_owned(),
        gl::FRAGMENT_SHADER => "fragment".to_owned(),
        gl::COMPUTE_SHADER => "compute".to_owned(),
        gl::GEOMETRY_SHADER => "geometry".to_owned(),
        gl::TESS_CONTROL_SHADER => "tesselation control".to_owned(),
        gl::TESS_EVALUATION_SHADER => "tesselation evalution".to_owned(),
        x => format!("unknown ({x})"),
    }
}

fn c_char_buf_to_string(buf: &[c_char], len: i32) -> String {
    let to_u8 = buf.iter().map(|i| *i as u8).collect::<Vec<_>>();
    let trim = usize::min(to_usize(len), buf.len());
    let slice = &to_u8[..trim];

    String::from_utf8_lossy(slice).to_string()
}

fn check_link_status(prog: u32) {
    unsafe {
        let mut success = 0;
        gl::GetProgramiv(prog, gl::LINK_STATUS, &mut success);

        if success == 1 {
            return;
        }

        let mut buf = vec![0; 512];
        let mut len = 0;
        gl::GetProgramInfoLog(prog, to_i32(buf.len()), &mut len, buf.as_mut_ptr());

        let log = c_char_buf_to_string(&buf, len);

        panic!("failed to link shader program\n{log}");
    }
}

pub fn init_gl() {
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::CULL_FACE);
        gl::Enable(gl::SCISSOR_TEST);

        gl::Enable(gl::BLEND);
        gl::BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);
        gl::BlendFuncSeparate(gl::ONE, gl::ONE_MINUS_SRC_ALPHA, gl::ONE_MINUS_DST_ALPHA, gl::ONE);

        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST_MIPMAP_LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
    }
}
