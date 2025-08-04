#version 430 core

layout(location = 0) in vec2 fragUV;
layout(location = 1) in vec4 fragColor;
layout(location = 2) flat in uint fragTexLayer;
layout(location = 3) flat in vec2 fragUVScale;
layout(location = 4) flat in vec4 fragScissor;

layout(location = 0) out vec4 outColor;

uniform sampler2DArray texArray;

void main() {
    if (gl_FragCoord.x < fragScissor.x
        || gl_FragCoord.y < fragScissor.y
        || gl_FragCoord.x > fragScissor.x + fragScissor.z
        || gl_FragCoord.y > fragScissor.y + fragScissor.w) {
        discard;
    }

    outColor = fragColor * texture(texArray, vec3(fragUV * fragUVScale, fragTexLayer));
}
