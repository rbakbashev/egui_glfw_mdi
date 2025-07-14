pub use imp::*;

#[cfg(not(feature = "tracy-client"))]
mod imp {
    pub fn setup_profiler() {}

    macro_rules! profile {
        () => {};
        ($name: expr) => {};
    }

    pub(crate) use profile;

    pub fn mark_frame_end() {}
}

#[cfg(feature = "tracy-client")]
mod imp {
    pub fn setup_profiler() {
        tracy_client::register_demangler!();
        tracy_client::set_thread_name!("Main thread");
    }

    macro_rules! profile {
        () => {
            let _s = tracy_client::span!();
        };
        ($name: expr) => {
            let _s = tracy_client::span!($name);
        };
    }

    pub(crate) use profile;

    pub fn mark_frame_end() {
        tracy_client::frame_mark();
    }
}
