#version 450
layout (location = 0) in vec2 vertex_position;
layout (location = 1) in vec4 vertex_color;

layout (location = 0) out vec4 fragment_color;

layout(set = 0, binding = 0)
uniform Uniform{ 
    mat4 view_matrix;
} uniforms;

void main(){
    fragment_color = vertex_color;
   gl_Position = uniforms.view_matrix * vec4(vertex_position.x, vertex_position.y, 0.0, 1.0);       
}
