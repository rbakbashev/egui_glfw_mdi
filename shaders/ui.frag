#version 430 core

layout(location = 0) in vec2 fragUV;
layout(location = 1) in vec4 fragColor;
layout(location = 2) flat in uint fragTexLayer;
layout(location = 3) flat in vec2 fragUVScale;

layout(location = 0) out vec4 outColor;

uniform sampler2DArray texArray;

void main() {
    outColor = fragColor * texture(texArray, vec3(fragUV * fragUVScale, fragTexLayer));
}
