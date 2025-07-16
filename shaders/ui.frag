#version 430 core

layout(location = 0) in vec2 fragUV;
layout(location = 1) in vec4 fragColor;

layout(location = 0) out vec4 outColor;

uniform sampler2DArray texArray;
uniform int texLayer;
uniform vec2 uvScale;

void main() {
    outColor = fragColor * texture(texArray, vec3(fragUV * uvScale, texLayer));
}
