#version 460 core

struct DrawElementsCmd {
    uint  count;
    uint  instanceCount;
    uint  firstIndex;
    int   baseVertex;
    uint  textureLayer;
    float uvScaleX;
    float uvScaleY;
    float scissorX;
    float scissorY;
    float scissorW;
    float scissorH;
};

layout(std430, binding = 0) readonly restrict buffer ssbo {
    DrawElementsCmd cmds[];
};

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 color;

layout(location = 0) out vec2 fragUV;
layout(location = 1) out vec4 fragColor;
layout(location = 2) flat out uint fragTexLayer;
layout(location = 3) flat out vec2 fragUVScale;
layout(location = 4) flat out vec4 fragScissor;

uniform vec2 screenSize;

void main() {
    fragUV       = uv;
    fragColor    = color / 255.;
    fragTexLayer = cmds[gl_DrawID].textureLayer;
    fragUVScale  = vec2(cmds[gl_DrawID].uvScaleX, cmds[gl_DrawID].uvScaleY);
    fragScissor  = vec4(
        cmds[gl_DrawID].scissorX,
        cmds[gl_DrawID].scissorY,
        cmds[gl_DrawID].scissorW,
        cmds[gl_DrawID].scissorH
    );

    gl_Position = vec4(
        2. * pos.x / screenSize.x - 1.,
        1. - 2. * pos.y / screenSize.y,
        0.,
        1.
    );
}
