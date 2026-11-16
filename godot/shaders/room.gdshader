// Procedural surfaces of the war room: 0 wood, 1 stone floor, 2 wall bricks, 3 cloth.
shader_type spatial;
render_mode cull_back, specular_disabled;
uniform int kind = 0;
uniform vec3 tint : source_color = vec3(0.4, 0.26, 0.14);
uniform float scale = 1.0;
varying vec3 wp;
varying vec3 wn;
float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float noise(vec2 p) {
	vec2 i = floor(p); vec2 f = fract(p); f = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1, 0)), f.x), mix(hash(i + vec2(0, 1)), hash(i + vec2(1, 1)), f.x), f.y);
}
void vertex() {
	wp = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	wn = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}
void fragment() {
	vec3 n = abs(wn);
	vec2 p = n.y > 0.5 ? wp.xz : (n.x > 0.5 ? wp.zy : wp.xy);
	p *= scale;
	vec3 c = tint;
	if (kind == 0) {
		// planks along x with grain
		float plank = floor(p.y / 1.6);
		float g = noise(vec2(p.x * 0.35 + plank * 7.0, p.y * 6.0));
		g = 0.75 + 0.3 * g + 0.08 * sin(p.x * 0.9 + g * 6.0 + plank);
		float seam = smoothstep(0.0, 0.05, abs(fract(p.y / 1.6) - 0.5) * 2.0 - 0.0);
		seam = 1.0 - 0.35 * (1.0 - smoothstep(0.0, 0.06, fract(p.y / 1.6))) ;
		c *= g * seam * (0.92 + 0.16 * hash(vec2(plank, 3.0)));
	} else if (kind == 1) {
		vec2 t = p / 6.0;
		vec2 id = floor(t); vec2 f = fract(t);
		float edge = min(min(f.x, 1.0 - f.x), min(f.y, 1.0 - f.y));
		float joint = smoothstep(0.0, 0.035, edge);
		float v = 0.8 + 0.3 * hash(id) + 0.18 * noise(p * 1.3);
		c *= v * mix(0.45, 1.0, joint);
	} else if (kind == 2) {
		vec2 t = vec2(p.x / 5.0, p.y / 2.4);
		float row = floor(t.y);
		t.x += mod(row, 2.0) * 0.5;
		vec2 id = floor(t); vec2 f = fract(t);
		float edge = min(min(f.x, 1.0 - f.x) * 5.0 / 2.4, min(f.y, 1.0 - f.y));
		float joint = smoothstep(0.0, 0.08, edge);
		float v = 0.78 + 0.3 * hash(id) + 0.2 * noise(p * 1.1);
		c *= v * mix(0.4, 1.0, joint);
		// darker towards the ceiling
		c *= mix(1.0, 0.45, clamp((wp.y + 8.0) / 34.0, 0.0, 1.0));
	} else {
		float w = 0.92 + 0.08 * sin(p.x * 14.0) * sin(p.y * 14.0);
		c *= w;
	}
	ALBEDO = c;
	ROUGHNESS = 0.9;
}
