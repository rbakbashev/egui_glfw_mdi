This is a rendering demo showcasing how to render entire [egui](https://github.com/emilk/egui) in
one draw call (`glMultiDrawElementsIndirect`), in contrast to the usual implementations that call
`glDrawElements` for each mesh. Source code for that simple integration is also included for
comparison purposes.

In an (admittedly, quite a synthetic) benchmark of about 400 draw calls:

| Implementation        | Mean    | Median  | Std.dev   |
| --------------------- | ------- | ------- | --------- |
| Simple                | 4.35 ms | 4.17 ms | 1.07 ms   |
| Simple + texture pool | 4.18 ms | 4.01 ms | 1.02 ms   |
| MDI                   | 1.77 ms | 1.47 ms | 916.57 μs |

Measured by [tracy](https://github.com/wolfpld/tracy):
  `cargo build --profile=relwithdbg --features=tracy-client`

Limitations

  * Requires "modern" OpenGL 4.6 (from 2017), and is not available in browser
    (needs [`WEBGL_multi_draw`](https://developer.mozilla.org/en-US/docs/Web/API/WEBGL_multi_draw))

  * Need to know maximum size of textures used in UI upfront.

  * Not a reusable library, because it's too tightly coupled to custom GLFW bindings and the `gl`
    crate.

  * I definitely screwed up high-DPI rendering and alpha blending somewhere.
