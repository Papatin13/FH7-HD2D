shader_type canvas_item;
uniform vec4 col : source_color = vec4(0.0, 0.0, 0.0, 0.0);
void fragment() { COLOR = col; }
