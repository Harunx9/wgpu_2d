#version 450
layout (location = 0) in vec2 fragment_tex_coords;
layout (location = 1) in vec4 fragment_color;

layout (location = 0) out vec4 color;

layout (set = 0, binding = 0) uniform texture2D bind_texture;
layout (set = 0, binding = 1) uniform sampler bind_sampler;

void main()
{
    color = fragment_color * 
        texture(sampler2D(bind_texture, bind_sampler), fragment_tex_coords);
}