#version 330 core
in vec2 TEXCOORD;
out vec4 color;
uniform sampler2D scene;
uniform vec3 tint;
uniform float strength;
void main() {
    vec4 original = texture(scene, TEXCOORD);
    color = vec4(mix(original.rgb, original.rgb * tint, strength), original.a);
}
