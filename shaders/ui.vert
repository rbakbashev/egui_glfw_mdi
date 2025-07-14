#version 430 core

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 color;

layout(location = 0) out vec2 fragUV;
layout(location = 1) out vec4 fragColor;

uniform vec2 screenSize;

void main() {
    fragUV    = uv;
    fragColor = color / 255.;

    gl_Position = vec4(
        2. * pos.x / screenSize.x - 1.,
        1. - 2. * pos.y / screenSize.y,
        0.,
        1.
    );
}
