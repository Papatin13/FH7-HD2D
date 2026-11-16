shader_type spatial;
render_mode unshaded, depth_draw_never, blend_mix, cull_disabled;
void fragment() {
	float r = length(UV - 0.5) * 2.0;
	ALBEDO = vec3(0.0);
	ALPHA = (1.0 - smoothstep(0.35, 1.0, r)) * 0.45;
}
