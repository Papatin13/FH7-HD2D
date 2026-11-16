## Trade screen: the two item windows and the name plates drawn natively at the
## places of the original ones (so the existing mouse handling still fits).
## The portraits stay the game's own sprites. Coordinates come in frame pixels.
extends Control
const Ui = preload("res://scripts/ui.gd")

const TOP := Color(0.33, 0.42, 0.9)
const BOTTOM := Color(0.17, 0.23, 0.62)
const RIM_LIGHT := Color(0.92, 0.94, 1.0)
const RIM_DARK := Color(0.1, 0.12, 0.3)
const TEXT := Color(1, 1, 1)
const VALUE := Color(0.75, 0.94, 1.0)
const SHADOW := Color(0.04, 0.05, 0.18, 0.8)
const PARCH := Color(0.94, 0.89, 0.72)
const PARCH_DARK := Color(0.62, 0.5, 0.3)
const INK := Color(0.26, 0.16, 0.08)

var font: Font
var title_font: Font
var ink_font: Font
var data := {}
var icon_cb: Callable    # icon number -> Texture2D
var icons := {}
var t := 0.0

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_IGNORE

func show_trade(d: Dictionary) -> void:
	data = d
	visible = not d.is_empty()
	if visible:
		queue_redraw()

func _process(delta: float) -> void:
	t += delta
	if visible:
		queue_redraw()

func _icon(n: int) -> Texture2D:
	if not icons.has(n) and icon_cb.is_valid():
		icons[n] = icon_cb.call(n)
	return icons.get(n)

func _box(r: Rect2, c: Color, rad: float) -> void:
	var sb := StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(int(rad))
	sb.anti_aliasing = true
	draw_style_box(sb, r)

func _grad(r: Rect2, top: Color, bottom: Color, rad: int) -> void:
	var bands := 8
	for i in bands:
		var sb := StyleBoxFlat.new()
		sb.bg_color = top.lerp(bottom, float(i) / (bands - 1))
		if i == 0:
			sb.corner_radius_top_left = rad
			sb.corner_radius_top_right = rad
		if i == bands - 1:
			sb.corner_radius_bottom_left = rad
			sb.corner_radius_bottom_right = rad
		var h := r.size.y / bands
		draw_style_box(sb, Rect2(r.position.x, r.position.y + h * i, r.size.x, h + 0.7))

func _txt(p: Vector2, s: String, fs: int, col: Color, k: float, align := 0, f: Font = null, shadow := true) -> void:
	var ff: Font = f if f else font
	if ink_font and col.v < 0.5:
		ff = ink_font
	var w := ff.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	var x := p.x - (w if align == 2 else (w * 0.5 if align == 1 else 0.0))
	draw_string(ff, Vector2(x, p.y), s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, col)

func _draw() -> void:
	if font == null or data.is_empty():
		return
	var s := size.y / 160.0
	var cur: PackedInt32Array = data.get("cursor", PackedInt32Array([-1, -1]))
	var picked: PackedInt32Array = data.get("picked", PackedInt32Array([-1, -1]))
	for side in 2:
		var sd: Dictionary = data["left" if side == 0 else "right"]
		var ri: Rect2i = sd.rect
		var r := Rect2(Vector2(ri.position) * s, Vector2(ri.size) * s).grow(-1 * s)
		# window
		Ui.draw(self, "win", r.grow(1 * s), s)
		var items: Array = sd.items
		for i in 5:
			var ry := r.position.y + (7 + 16 * i) * s
			var row := Rect2(r.position.x + 4 * s, ry, r.size.x - 8 * s, 15 * s)
			if cur[0] == side and cur[1] == i:
				Ui.bar(self, Rect2(row.position.x + 16 * s, row.position.y, row.size.x - 16 * s, row.size.y), s)
			if picked[0] == side and picked[1] == i:
				_box(row, Color(1.0, 0.86, 0.35, 0.45), 3 * s)
			if i >= items.size():
				continue
			var it: Dictionary = items[i]
			var ic := _icon(int(it.icon))
			if ic:
				draw_texture_rect(ic, Rect2(Vector2(row.position.x + 1 * s, ry - 0.5 * s), Vector2(16, 16) * s), false)
			_txt(Vector2(row.position.x + 19 * s, ry + 11.5 * s), str(it.name), int(11.5 * s), TEXT, s)
			var u: int = it.uses
			_txt(Vector2(row.end.x - 2 * s, ry + 12 * s), "--" if u < 0 else str(u), int(12 * s), VALUE, s, 2)
		# name plate above, at the outer top corner like the original
		var pw := 64.0 * s
		var px := 2.0 * s if side == 0 else size.x - pw - 2.0 * s
		var plate := Rect2(Vector2(px, -3 * s), Vector2(pw, 20 * s))
		_box(plate, Color(0.22, 0.42, 0.25, 0.6), 4 * s)
		_txt(Vector2(plate.get_center().x, plate.position.y + 16 * s), str(sd.name), int(12 * s), TEXT, s, 1)
