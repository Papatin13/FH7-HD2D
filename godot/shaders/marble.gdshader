// Green marble backdrop (like the original personal-information screen).
shader_type canvas_item;
uniform vec2 size = vec2(1600.0, 900.0);
float h(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float n(vec2 p) {
	vec2 i = floor(p), f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(h(i), h(i + vec2(1, 0)), u.x), mix(h(i + vec2(0, 1)), h(i + vec2(1, 1)), u.x), u.y);
}
float fbm(vec2 p) {
	float v = 0.0, a = 0.5;
	for (int k = 0; k < 6; k++) { v += a * n(p); p *= 2.03; a *= 0.5; }
	return v;
}
void fragment() {
	vec2 p = UV * size / 220.0;
	float t = TIME * 0.01;
	float f = fbm(p + vec2(t, 0.0));
	float veins = abs(sin((p.x * 0.8 + p.y * 1.3) * 2.2 + f * 7.0));
	veins = pow(1.0 - veins, 3.0) * smoothstep(0.3, 0.7, fbm(p * 0.7 + 9.0));
	float g = fbm(p * 2.5 + 3.0);
	vec3 dark = vec3(0.24, 0.45, 0.27);
	vec3 mid = vec3(0.47, 0.70, 0.45);
	vec3 light = vec3(0.86, 0.95, 0.84);
	vec3 c = mix(dark, mid, smoothstep(0.25, 0.75, g));
	c = mix(c, light, veins * 0.55);
	c = mix(c, light * 0.95, smoothstep(0.55, 0.9, fbm(p * 1.3 + 5.0)) * 0.35);
	c *= 0.9 + 0.2 * f;
	// vignette
	vec2 d = UV - 0.5;
	c *= 1.0 - dot(d, d) * 0.9;
	COLOR = vec4(c, 1.0);
}
