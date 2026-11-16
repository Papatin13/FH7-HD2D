// HD-2D tilt-shift: sharp band across the middle, top and bottom softly blurred.
shader_type canvas_item;
uniform sampler2D screen_tex : hint_screen_texture, filter_linear_mipmap;
uniform float strength = 0.7;
uniform float band_center = 0.52;
uniform float band_half = 0.3;
void fragment() {
	float d = max(abs(SCREEN_UV.y - band_center) - band_half, 0.0);
	float t = clamp(d * 4.0, 0.0, 1.0) * strength;
	if (t <= 0.001) {
		COLOR = vec4(textureLod(screen_tex, SCREEN_UV, 0.0).rgb, 1.0);
	} else {
		vec2 px = SCREEN_PIXEL_SIZE * (1.0 + 5.0 * t);
		float lod = t * 2.0;
		vec3 acc = vec3(0.0);
		float wsum = 0.0;
		for (int i = -2; i <= 2; i++) {
			for (int j = -2; j <= 2; j++) {
				float w = exp(-float(i * i + j * j) * 0.35);
				acc += textureLod(screen_tex, SCREEN_UV + vec2(float(i), float(j)) * px, lod).rgb * w;
				wsum += w;
			}
		}
		COLOR = vec4(acc / wsum, 1.0);
	}
}
