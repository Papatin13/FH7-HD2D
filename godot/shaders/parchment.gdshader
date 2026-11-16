// Torn parchment with a rolled left edge (like the original scroll).
shader_type canvas_item;
uniform vec2 size = vec2(1180.0, 740.0);
float h(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }
float n(vec2 p) {
	vec2 i = floor(p), f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(h(i), h(i + vec2(1, 0)), u.x), mix(h(i + vec2(0, 1)), h(i + vec2(1, 1)), u.x), u.y);
}
float fbm(vec2 p) {
	float v = 0.0, a = 0.5;
	for (int k = 0; k < 5; k++) { v += a * n(p); p *= 2.1; a *= 0.5; }
	return v;
}
void fragment() {
	vec2 px = UV * size;
	float roll = 86.0;
	// distance to the torn border (the rolled left side is straight)
	float edge_noise = (fbm(px / 26.0) - 0.5) * 34.0 + (n(px / 7.0) - 0.5) * 8.0;
	float dl = px.x - roll * 0.5;
	float dr = size.x - px.x;
	float dt = px.y;
	float db = size.y - px.y;
	float d = min(min(dr, dt), db) + edge_noise - 18.0;
	float alpha = smoothstep(0.0, 2.0, d);
	vec3 base = mix(vec3(0.93, 0.74, 0.43), vec3(0.85, 0.6, 0.3), fbm(px / 140.0));
	base *= 0.9 + 0.18 * fbm(px / 40.0 + 7.0);
	// stains
	base = mix(base, vec3(0.72, 0.48, 0.22), smoothstep(0.62, 0.8, fbm(px / 90.0 + 13.0)) * 0.5);
	// burnt border
	float burn = 1.0 - smoothstep(0.0, 34.0, d);
	base = mix(base, vec3(0.42, 0.22, 0.08), burn * 0.85);
	// rolled left edge: a cylinder
	if (px.x < roll) {
		float x = px.x / roll;
		// cylinder: light from the upper left, dark seam where it meets the sheet
		float shade = 0.45 + 0.75 * pow(sin(x * 3.14159), 0.7) - 0.25 * x;
		vec3 rc = mix(vec3(0.93, 0.74, 0.43), vec3(0.8, 0.55, 0.27), fbm(vec2(px.x / 30.0, px.y / 160.0))) * shade;
		rc += vec3(0.25, 0.2, 0.12) * smoothstep(0.15, 0.3, x) * (1.0 - smoothstep(0.3, 0.45, x));
		rc = mix(rc, vec3(0.3, 0.15, 0.05), smoothstep(0.85, 1.0, x) * 0.7);
		float top = 30.0 + (fbm(vec2(0.0, px.x / 20.0)) - 0.5) * 20.0;
		float bot = size.y - 40.0 + (fbm(vec2(3.0, px.x / 20.0)) - 0.5) * 20.0;
		alpha = smoothstep(top - 1.0, top + 1.0, px.y) * (1.0 - smoothstep(bot - 1.0, bot + 1.0, px.y));
		base = rc;
	}
	COLOR = vec4(base, alpha);
}
