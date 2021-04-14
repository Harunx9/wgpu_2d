#version 450
layout (location = 0) in vec2 vertex_position;
layout (location = 1) in vec2 vertex_tex_coord;
layout (location = 2) in vec4 vertex_color;

layout (location = 0) out vec2 fragment_tex_coords;
layout (location = 1) out vec4 fragment_color;

layout(set=1, binding=0)
uniform Uniforms{ 
    mat4 camera_matrix;
} uniforms;

void main()
{
    fragment_tex_coords = vertex_tex_coord;
    fragment_color = vertex_color;
    gl_Position = uniforms.camera_matrix * vec4(vertex_position.x, vertex_position.x, 0.0f, 1.0f);
}
