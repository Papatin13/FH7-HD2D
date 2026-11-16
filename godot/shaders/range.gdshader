shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, blend_mix;
void fragment() {
	float p = 0.75 + 0.25 * sin(TIME * 3.0);
	vec2 d = abs(fract(UV) - 0.5);
	float edge = smoothstep(0.40, 0.48, max(d.x, d.y));
	ALBEDO = COLOR.rgb * (1.0 + 0.6 * edge);
	ALPHA = COLOR.a * (0.55 + 0.45 * edge) * p;
}
