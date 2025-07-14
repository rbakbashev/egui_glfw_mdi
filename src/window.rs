use std::ffi::{CStr, CString, c_char, c_int};
use std::ptr::null_mut;

#[allow(clippy::wildcard_imports)]
use glfw_sys::*;

use crate::main_loop::{Event, MainLoop};
use crate::utils::{CheckError, to_cstring, to_i32, to_u32};

pub struct Window {
    handle: *mut GLFWwindow,
    width: u32,
    height: u32,
}

#[allow(unused)]
#[derive(Clone, Copy)]
pub enum Resolution {
    Windowed(u32, u32),
    // rest are left out for brevity
}

impl Window {
    pub fn new(res: Resolution, monitor_idx: usize, title: &str) -> Self {
        init_glfw();

        let cstring = CString::new(title).try_to(format!("convert {title} to CString"));
        let handle = create_window(res, monitor_idx, cstring.as_c_str());
        let (width, height) = get_framebuffer_size(handle);

        disable_vsync();
        load_functions();

        Self { handle, width, height }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn set_event_dest(&self, ptr: *mut MainLoop) {
        let handle = self.handle;

        unsafe {
            glfwSetWindowUserPointer(handle, ptr.cast());

            glfwSetKeyCallback(handle, Some(key_callback));
            glfwSetFramebufferSizeCallback(handle, Some(fb_size_callback));
            glfwSetCursorPosCallback(handle, Some(mouse_pos_callback));
            glfwSetMouseButtonCallback(handle, Some(mouse_button_callback));
            glfwSetScrollCallback(handle, Some(mouse_scroll_callback));
        }
    }

    pub fn poll_events(&self) {
        unsafe {
            glfwPollEvents();
        }
    }

    pub fn should_close(&self) -> bool {
        unsafe { glfwWindowShouldClose(self.handle) != 0 }
    }

    pub fn set_viewport(&self) {
        unsafe {
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
        }
    }

    pub fn swap_buffers(&self) {
        unsafe {
            glfwSwapBuffers(self.handle);
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            glfwDestroyWindow(self.handle);
            glfwTerminate();
        }
    }
}

fn init_glfw() {
    unsafe {
        glfwSetErrorCallback(Some(error_callback));

        if glfwInit() == 0 {
            panic!("failed to initialize GLFW");
        }
    }
}

extern "C" fn error_callback(error_code: c_int, desc_ptr: *const c_char) {
    let desc = unsafe { CStr::from_ptr(desc_ptr) }.to_string_lossy();

    panic!("{desc} (GLFW {error_code:#x})");
}

fn create_window(res: Resolution, monitor_idx: usize, title: &CStr) -> *mut GLFWwindow {
    let monitor = get_monitor(monitor_idx);
    let (mw, mh) = get_monitor_res(monitor);
    let Resolution::Windowed(w, h) = res;

    set_windowed_hints(w, h, mw, mh);
    create_raw_window(w, h, title, null_mut())
}

fn get_monitor(idx: usize) -> *mut GLFWmonitor {
    let mut count = 0;
    let monitors = unsafe { glfwGetMonitors(&mut count) };

    if count == 0 {
        panic!("no monitors found");
    }

    if idx >= count as usize {
        panic!("monitor with index {idx} was requested, but only {count} were found");
    }

    unsafe { monitors.add(idx).read() }
}

fn get_monitor_res(monitor: *mut GLFWmonitor) -> (u32, u32) {
    let mode = get_video_mode(monitor);
    let w = to_u32(mode.width);
    let h = to_u32(mode.height);

    (w, h)
}

fn get_video_mode<'a>(monitor: *mut GLFWmonitor) -> &'a GLFWvidmode {
    unsafe { glfwGetVideoMode(monitor).as_ref() }.try_to("get monitor's video mode")
}

fn set_windowed_hints(w: u32, h: u32, mw: u32, mh: u32) {
    if w >= mw || h >= mh {
        return;
    }

    let pos_x = mw / 2 - w / 2;
    let pos_y = mh / 2 - h / 2;

    let pos_x_int = to_i32(pos_x);
    let pos_y_int = to_i32(pos_y);

    unsafe {
        glfwWindowHint(GLFW_POSITION_X, pos_x_int);
        glfwWindowHint(GLFW_POSITION_Y, pos_y_int);
    }
}

fn create_raw_window(w: u32, h: u32, title: &CStr, monitor: *mut GLFWmonitor) -> *mut GLFWwindow {
    let wi = to_i32(w);
    let hi = to_i32(h);

    unsafe {
        glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);
        glfwWindowHint(GLFW_CENTER_CURSOR, GLFW_TRUE);
        glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 4);
        glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 6);
        glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);

        if cfg!(debug_assertions) {
            glfwWindowHint(GLFW_CONTEXT_DEBUG, GLFW_TRUE);
        }

        let window = glfwCreateWindow(wi, hi, title.as_ptr().cast(), monitor, null_mut())
            .try_to("create window");

        glfwMakeContextCurrent(window);

        window
    }
}

fn get_framebuffer_size(window: *mut GLFWwindow) -> (u32, u32) {
    let mut wi = 0;
    let mut hi = 0;

    unsafe { glfwGetFramebufferSize(window, &mut wi, &mut hi) };

    let w = to_u32(wi);
    let h = to_u32(hi);

    (w, h)
}

fn disable_vsync() {
    unsafe {
        glfwSwapInterval(0);
    }
}

fn load_functions() {
    gl::load_with(|func| {
        let cstr = to_cstring(func);

        unsafe {
            glfwGetProcAddress(cstr.as_ptr()).try_to(format!("find function {func:?}")) as *const _
        }
    });
}

extern "C" fn key_callback(handle: *mut GLFWwindow, code: i32, _sc: i32, action: i32, _mods: i32) {
    let key = unsafe { std::mem::transmute::<i32, Key>(code) };

    match action {
        GLFW_PRESS => call_handler(handle, Event::KeyPress(key)),
        GLFW_RELEASE => call_handler(handle, Event::KeyRelease(key)),
        _ => {}
    }
}

extern "C" fn fb_size_callback(handle: *mut GLFWwindow, w: i32, h: i32) {
    let wu = to_u32(w);
    let hu = to_u32(h);
    let window = main_loop_mut(handle).window_mut();

    window.width = wu;
    window.height = hu;

    call_handler(handle, Event::WindowResize(wu, hu));
}

extern "C" fn mouse_pos_callback(handle: *mut GLFWwindow, x: f64, y: f64) {
    call_handler(handle, Event::MouseMove(x as f32, y as f32));
}

extern "C" fn mouse_button_callback(handle: *mut GLFWwindow, button: i32, action: i32, _mods: i32) {
    let num = button + 1;

    match action {
        GLFW_PRESS => call_handler(handle, Event::MousePress(num)),
        GLFW_RELEASE => call_handler(handle, Event::MouseRelease(num)),
        _ => {}
    }
}

extern "C" fn mouse_scroll_callback(handle: *mut GLFWwindow, x: f64, y: f64) {
    call_handler(handle, Event::MouseScroll(x as f32, y as f32));
}

fn call_handler(handle: *mut GLFWwindow, event: Event) {
    main_loop_mut(handle).handle_event(event);
}

fn main_loop_mut<'a>(handle: *mut GLFWwindow) -> &'a mut MainLoop {
    unsafe {
        glfwGetWindowUserPointer(handle).cast::<MainLoop>().as_mut().or_err("window userptr unset")
    }
}
