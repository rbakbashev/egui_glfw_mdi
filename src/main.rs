#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::suboptimal_flops
)]
#![allow(
    clippy::borrow_as_ptr,
    clippy::identity_op,
    clippy::manual_assert,
    clippy::many_single_char_names,
    clippy::missing_const_for_fn,
    clippy::option_if_let_else,
    clippy::similar_names,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps,
    clippy::unused_self
)]

mod gl;
mod main_loop;
mod profiler;
mod ui;
mod utils;
mod window;

use main_loop::MainLoop;
use profiler::setup_profiler;

fn main() {
    setup_profiler();

    MainLoop::new().run();
}
