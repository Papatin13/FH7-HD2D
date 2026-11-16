shader_type spatial;
render_mode unshaded, cull_disabled, depth_test_disabled, blend_mix;
uniform vec4 tint : source_color = vec4(1.0, 0.85, 0.3, 1.0);
void fragment() {
	vec2 d = abs(UV - 0.5);
	float m = max(d.x, d.y);
	float corner = step(0.26, min(d.x, d.y));
	float ring = step(0.37, m) * (1.0 - step(0.5, m));
	float inner = step(0.40, m) * (1.0 - step(0.47, m));
	float pulse = 0.8 + 0.2 * sin(TIME * 6.0);
	vec3 c = mix(vec3(0.15, 0.08, 0.0), tint.rgb * 1.5, inner);
	ALBEDO = c;
	ALPHA = ring * corner * pulse + (1.0 - corner) * (1.0 - smoothstep(0.0, 0.5, m)) * 0.10;
}
