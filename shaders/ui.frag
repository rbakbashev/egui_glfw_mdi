#version 430 core

layout(location = 0) in vec2 fragUV;
layout(location = 1) in vec4 fragColor;

layout(location = 0) out vec4 outColor;

uniform sampler2D tex;

void main() {
    outColor = fragColor * texture(tex, fragUV);
}
