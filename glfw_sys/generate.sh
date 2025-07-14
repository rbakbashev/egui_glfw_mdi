#!/bin/sh

header=$(cat <<EOF
#![allow(bad_style, clippy::pub_underscore_fields, clippy::unreadable_literal, clippy::use_self)]

use std::ffi;

#[cfg_attr(target_os = "linux", link(name = "glfw", kind = "dylib"))]
#[cfg_attr(target_os = "windows", link(name = "glfw3dll", kind = "dylib"))]
unsafe extern "C" {}
EOF
)

bindgen \
    --default-macro-constant-type signed \
    --no-doc-comments \
    --no-layout-tests \
    --no-derive-debug \
    --merge-extern-blocks \
    --no-prepend-enum-name \
    --ctypes-prefix 'ffi' \
    --default-enum-style rust \
    --rust-target 1.89.0 \
    --rust-edition 2024 \
    --rustfmt-configuration-file "$(pwd)/../rustfmt.toml" \
    --raw-line "$header" \
    wrapper.h \
    -o lib.rs
