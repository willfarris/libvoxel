#version 310 es

precision mediump float;

in vec3 v_normal;
in vec3 v_position;
in vec2 v_tex_coords;

uniform vec3 sunlight_direction;
uniform sampler2D texture_map;

layout(location = 0) out vec4 color;

void main() {
    vec4 tex_color = texture(texture_map, v_tex_coords).rgba;
    if(tex_color.a < 0.5) { discard; }

    float diffuse = max(dot(normalize(v_normal), normalize(sunlight_direction)), 0.5);

    vec3 camera_dir = normalize(-v_position);
    vec3 half_direction = normalize(normalize(sunlight_direction) + camera_dir);
    float specular = pow(max(dot(half_direction, normalize(v_normal)), 0.0), 16.0);
    tex_color.rgb = tex_color.rgb + specular * tex_color.rgb * vec3(0.9);

    color = vec4(vec3(0.01) + diffuse * tex_color.rgb, tex_color.a);
}