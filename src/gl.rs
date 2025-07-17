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

pub struct TextureArray {
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

    pub fn set_ssbo_binding(&self, idx: u32) {
        unsafe {
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, idx, self.id);
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

impl TextureArray {
    pub fn new(internal_format: u32, w: i32, h: i32, d: i32) -> Self {
        let mut id = 0;

        unsafe {
            gl::GenTextures(1, &mut id);
            gl::BindTexture(gl::TEXTURE_2D_ARRAY, id);
            gl::TexStorage3D(gl::TEXTURE_2D_ARRAY, 1, internal_format, w, h, d);
        }

        Self { id }
    }

    pub fn enable(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D_ARRAY, self.id);
        }
    }

    pub fn upload<T>(&self, x: i32, y: i32, z: i32, w: usize, h: usize, fmt: u32, data: &[T]) {
        let w = w as i32;
        let h = h as i32;
        let ty = gl::UNSIGNED_BYTE;
        let pixels = data.as_ptr().cast();

        unsafe {
            gl::TexSubImage3D(gl::TEXTURE_2D_ARRAY, 0, x, y, z, w, h, 1, fmt, ty, pixels);
        }
    }

    pub fn generate_mipmaps(&self) {
        unsafe {
            gl::GenerateMipmap(gl::TEXTURE_2D_ARRAY);
        }
    }
}

impl Drop for TextureArray {
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

        let min = gl::NEAREST_MIPMAP_LINEAR as i32;
        let mag = gl::NEAREST as i32;

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, mag);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, min);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

        gl::TexParameteri(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_MAG_FILTER, mag);
        gl::TexParameteri(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_MIN_FILTER, min);
        gl::TexParameteri(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
    }
}
