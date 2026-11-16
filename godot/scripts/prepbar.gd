## Buttons of the "Check Map" view during the preparations (instead of the game's
## START / B / R / SELECT hints): with its menu open "Fight!", "Back", "Info";
## on the free map a "Menu" button in the top right corner.
extends Control
const Ui = preload("res://scripts/ui.gd")

signal key(gba: int)

const KB := 2
const KSELECT := 4
const KSTART := 8
const KR := 256
const TOP := Color(0.3, 0.38, 0.85)
const BOTTOM := Color(0.18, 0.24, 0.66)
const RIM := Color(0.9, 0.93, 1.0)

var font: Font
var title_font: Font
var state := 0
var labels := {}
var buttons := []
var hover := -1

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_IGNORE

func show_state(s: int) -> void:
	if s != state:
		state = s
		queue_redraw()
	visible = s != 0

func _L(k: String) -> String:
	return str(labels.get(k, k))

func _layout() -> Array:
	var k := size.y / 900.0
	var out := []
	if state == 1:
		var w := 300.0 * k
		var h := 84.0 * k
		var y := size.y - h - 24 * k
		var x0 := size.x * 0.5 - (3 * w + 2 * 24 * k) * 0.5
		out.append([Rect2(x0, y, w, h), _L("Fight!"), KSTART, true])
		out.append([Rect2(x0 + w + 24 * k, y, w, h), _L("Back"), KB, false])
		out.append([Rect2(x0 + 2 * (w + 24 * k), y, w, h), _L("Info"), KR, false])
	elif state == 2:
		var w := 230.0 * k
		var h := 70.0 * k
		out.append([Rect2(size.x - w - 24 * k, 24 * k, w, h), _L("Menu"), KSELECT, false])
	return out

func _input(ev: InputEvent) -> void:
	if not visible:
		return
	if ev is InputEventMouseMotion:
		var h := -1
		var bs := _layout()
		for i in bs.size():
			if (bs[i][0] as Rect2).has_point(ev.position - global_position):
				h = i
		if h != hover:
			hover = h
			queue_redraw()
	elif ev is InputEventMouseButton and ev.pressed and ev.button_index == MOUSE_BUTTON_LEFT:
		for b in _layout():
			if (b[0] as Rect2).has_point(ev.position - global_position):
				key.emit(b[2])
				get_viewport().set_input_as_handled()
				return

func _box(r: Rect2, c: Color, rad: int) -> void:
	var sb := StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(rad)
	sb.anti_aliasing = true
	draw_style_box(sb, r)

func _draw() -> void:
	if font == null:
		return
	var k := size.y / 900.0
	var bs := _layout()
	for i in bs.size():
		var r: Rect2 = bs[i][0]
		var red: bool = bs[i][3]
		var hot := i == hover
		Ui.draw(self, "barsel" if (hot or red) else "bar", r, r.size.y / 23.0)
		var f: Font = font
		var fs := int(38 * k)
		var s: String = bs[i][1]
		var w := f.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
		var p := Vector2(r.get_center().x - w * 0.5, r.position.y + r.size.y * 0.66)
		draw_string(f, p, s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Color(1, 1, 1))
