## Phase change ("Player Phase") and chapter title banners, drawn natively in
## place of the game's (which are hidden while these are up).
extends Control

const PHASE_COL := {
	"Player Phase": [Color(0.3, 0.5, 1.0), Color(0.75, 0.88, 1.0)],
	"Enemy Phase": [Color(0.95, 0.25, 0.25), Color(1.0, 0.8, 0.7)],
	"Other Phase": [Color(0.3, 0.8, 0.4), Color(0.8, 1.0, 0.8)],
}
const GOLD := Color(1.0, 0.86, 0.45)

var title_font: Font
var data := {}

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_IGNORE

func show_banner(d: Dictionary) -> void:
	data = d
	visible = not d.is_empty()
	if visible:
		queue_redraw()

func _draw() -> void:
	if data.is_empty() or title_font == null:
		return
	var s := size.y / 160.0
	var f: float = data.frames
	var text := str(data.text)
	if int(data.kind) == 0:
		_phase(text, f, s)
	else:
		_title(text, f, s)

func _phase(text: String, f: float, s: float) -> void:
	var cols: Array = PHASE_COL.get(text, PHASE_COL["Player Phase"])
	var a := clampf(f / 10.0, 0.0, 1.0)
	var cy := size.y * 0.5
	var h := 34.0 * s * a
	# band with soft edges
	for i in 12:
		var k := float(i) / 11.0
		var hh := h * (1.0 - k * 0.6)
		draw_rect(Rect2(0, cy - hh * 0.5, size.x, hh), Color(cols[0].r * 0.35, cols[0].g * 0.35, cols[0].b * 0.35, 0.12 + 0.05 * (1.0 - k)))
	draw_rect(Rect2(0, cy - h * 0.5, size.x, 1.5 * s), Color(cols[1], 0.8 * a))
	draw_rect(Rect2(0, cy + h * 0.5 - 1.5 * s, size.x, 1.5 * s), Color(cols[1], 0.8 * a))
	# text slides in from the side and settles
	var fs := int(26 * s)
	var w := title_font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	var slide := pow(1.0 - clampf(f / 14.0, 0.0, 1.0), 3.0)
	var x := size.x * 0.5 - w * 0.5 - slide * size.x * 0.6
	var y := cy + 9 * s
	draw_string(title_font, Vector2(x, y) + Vector2(2, 2) * s, text, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Color(0, 0, 0, 0.6 * a))
	draw_string_outline(title_font, Vector2(x, y), text, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, int(3 * s), Color(cols[0].darkened(0.5), a))
	draw_string(title_font, Vector2(x, y), text, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Color(cols[1], a))

func _title(text: String, f: float, s: float) -> void:
	var a := clampf(f / 20.0, 0.0, 1.0)
	draw_rect(Rect2(Vector2.ZERO, size), Color(0.02, 0.02, 0.05, 0.9))
	var parts := text.split(":", false, 1)
	var head := parts[0].strip_edges() if parts.size() > 1 else ""
	var main := parts[1].strip_edges() if parts.size() > 1 else text
	var cy := size.y * 0.5
	var fs := int(22 * s)
	var w := title_font.get_string_size(main, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	var pw := maxf(w + 60 * s, 160 * s)
	var plate := Rect2(size.x * 0.5 - pw * 0.5, cy - 22 * s, pw, 44 * s)
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.32, 0.12, 0.06, 0.95 * a)
	sb.border_color = Color(GOLD, a)
	sb.set_border_width_all(int(2 * s))
	sb.set_corner_radius_all(int(4 * s))
	draw_style_box(sb, plate)
	draw_rect(Rect2(plate.position + Vector2(6, 4) * s, Vector2(plate.size.x - 12 * s, 1 * s)), Color(GOLD, 0.5 * a))
	draw_rect(Rect2(plate.position + Vector2(6 * s, plate.size.y - 5 * s), Vector2(plate.size.x - 12 * s, 1 * s)), Color(GOLD, 0.5 * a))
	if head != "":
		var hs := int(11 * s)
		var hw := title_font.get_string_size(head, HORIZONTAL_ALIGNMENT_LEFT, -1, hs).x
		draw_string(title_font, Vector2(size.x * 0.5 - hw * 0.5, plate.position.y + 15 * s), head, HORIZONTAL_ALIGNMENT_LEFT, -1, hs, Color(GOLD, a))
	draw_string(title_font, Vector2(size.x * 0.5 - w * 0.5, plate.position.y + (36 if head != "" else 29) * s), main, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Color(1, 0.97, 0.9, a))
