This is a rendering demo showcasing how to render entire [egui](https://github.com/emilk/egui) in
one draw call (`glMultiDrawElementsIndirect`), in contrast to the usual implementations that call
`glDrawElements` for each mesh. Source code for that simple integration is also included for
comparison purposes.

In an (admittedly, quite a synthetic) benchmark of about 400 draw calls:

| Implementation        | Mean    | Median  | Std.dev   |
| --------------------- | ------- | ------- | --------- |
| Simple                | 4.13 ms | 4.05 ms | 1.22 ms   |
| Simple + texture pool | 3.94 ms | 3.79 ms | 1.13 ms   |
| MDI                   | 1.6 ms  | 1.22 ms | 935.95 μs |

Measured by [tracy](https://github.com/wolfpld/tracy):
  `cargo build --profile=relwithdbg --features=tracy-client`

Limitations

  * Requires "modern" OpenGL 4.6 (from 2017), and is not available in browser
    (needs [`WEBGL_multi_draw`](https://developer.mozilla.org/en-US/docs/Web/API/WEBGL_multi_draw))

  * Need to know maximum size of textures used in UI upfront.

  * Not a reusable library, because it's too tightly coupled to custom GLFW bindings and the `gl`
    crate.

  * I definitely screwed up high-DPI rendering and alpha blending somewhere.
