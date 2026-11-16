## World map narration box (between chapters), drawn natively at the place of
## the original one; the portrait above it stays the game's sprite.
extends Control
const Ui = preload("res://scripts/ui.gd")

const TOP := Color(0.33, 0.42, 0.9, 0.96)
const BOTTOM := Color(0.15, 0.2, 0.56, 0.96)
const RIM_LIGHT := Color(0.95, 0.9, 0.72)
const RIM_DARK := Color(0.32, 0.22, 0.1)
const TEXT := Color(1, 1, 1)
const SHADOW := Color(0.02, 0.03, 0.12, 0.85)

var font: Font
var lines := PackedStringArray()
var ext := 0.0

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_IGNORE

func show_lines(l: PackedStringArray, ext_left: int) -> void:
	lines = l
	ext = ext_left
	visible = l.size() > 0
	if visible:
		queue_redraw()

func _draw() -> void:
	if font == null or lines.is_empty():
		return
	var s := size.y / 160.0
	var fw := size.x / s
	var r := Rect2(Vector2(ext + 2, 113) * s, Vector2(fw - 2 * ext - 4, 45) * s)
	r = Rect2(Vector2(ext + 3, 115) * s, Vector2(fw - 2 * ext - 6, 43) * s)
	Ui.draw(self, "narr", r, s)
	var inner := r.grow(-4 * s)
	var re := RegEx.create_from_string("\\[[^\\]]*\\]")
	for i in mini(lines.size(), 2):
		var t := re.sub(lines[i], "", true)
		var p := Vector2(inner.position.x + 6 * s, inner.position.y + (15 + 16 * i) * s)
		draw_string(font, p, t, HORIZONTAL_ALIGNMENT_LEFT, -1, int(12 * s), TEXT)
