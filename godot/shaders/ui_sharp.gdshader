// Sharp bilinear: crisp pixels at any (non-integer) scale without shimmer.
shader_type canvas_item;
uniform vec2 tex_size = vec2(240.0, 160.0);
void fragment() {
	vec2 texel = UV * tex_size;
	vec2 scale = max(vec2(1.0), 1.0 / max(fwidth(texel), vec2(1e-4)));
	vec2 fl = floor(texel);
	vec2 cd = fract(texel) - 0.5;
	vec2 rr = 0.5 - 0.5 / scale;
	vec2 f = (cd - clamp(cd, -rr, rr)) * scale + 0.5;
	COLOR = texture(TEXTURE, (fl + f) / tex_size) * COLOR;
}
