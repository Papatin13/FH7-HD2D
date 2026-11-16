## The game's option screen ("Configuration"), drawn natively: all options at
## once in a blue panel, the chosen value of each highlighted, a help line at
## the bottom. Clicking a value moves the game's cursor there (its keys).
extends Control
const Ui = preload("res://scripts/ui.gd")

signal key(gba: int)

const W := 1600.0
const H := 900.0
const U := 5.625   # one original pixel on the 1600x900 stage
const FS := 1.45   # the game font runs smaller than the old one
const PANEL_TOP := Color(0.3, 0.38, 0.85)
const PANEL_BOTTOM := Color(0.18, 0.24, 0.66)
const RIM_LIGHT := Color(0.9, 0.93, 1.0)
const RIM_DARK := Color(0.1, 0.12, 0.3)
const LABEL := Color(1.0, 0.97, 0.45)
const ON := Color(1, 1, 1)
const OFF := Color(0.5, 0.53, 0.72)
const SHADOW := Color(0.04, 0.05, 0.18, 0.85)
const PARCH := Color(0.94, 0.89, 0.72)
const PARCH_DARK := Color(0.66, 0.56, 0.34)
const INK := Color(0.28, 0.18, 0.08)
const KA := 1
const KB := 2
const KRIGHT := 16
const KLEFT := 32
const KUP := 64
const KDOWN := 128

var font: Font
var title_font: Font
var ink_font: Font     # dialogue font: dark text on light boxes
var data := {}
var t := 0.0
var buttons := []

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_STOP

func show_options(d: Dictionary) -> void:
	data = d
	visible = not d.is_empty()
	if visible:
		queue_redraw()

func _process(delta: float) -> void:
	t += delta
	if visible:
		queue_redraw()

func _stage() -> Array:
	var k := minf(size.x / W, size.y / H)
	return [k, (size - Vector2(W, H) * k) * 0.5]

func _gui_input(ev: InputEvent) -> void:
	if not (ev is InputEventMouseButton and ev.pressed):
		return
	var st := _stage()
	var p: Vector2 = (ev.position - st[1]) / st[0]
	match ev.button_index:
		MOUSE_BUTTON_LEFT:
			for b in buttons:
				if (b[0] as Rect2).has_point(p):
					for k in b[1]:
						key.emit(k)
					accept_event()
					return
		MOUSE_BUTTON_RIGHT:
			key.emit(KB)
			accept_event()
		MOUSE_BUTTON_WHEEL_UP:
			key.emit(KUP)
			accept_event()
		MOUSE_BUTTON_WHEEL_DOWN:
			key.emit(KDOWN)
			accept_event()

func _box(r: Rect2, c: Color, rad := 10, border := Color(0, 0, 0, 0), bw := 0) -> void:
	var sb := StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(rad)
	sb.anti_aliasing = true
	if bw > 0:
		sb.border_color = border
		sb.set_border_width_all(bw)
	draw_style_box(sb, r)

func _grad(r: Rect2, top: Color, bottom: Color, rad: int) -> void:
	var bands := 10
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
		draw_style_box(sb, Rect2(r.position.x, r.position.y + h * i, r.size.x, h + 1))

func _txt(p: Vector2, s: String, fs: int, col: Color, f: Font = null, shadow := true, align := 0) -> float:
	var ff: Font = f if f else font
	if (f == null or f == font) and ink_font and col.v < 0.5:
		ff = ink_font
	if ff != title_font:
		fs = int(fs * FS)
	var w := ff.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	var x := p.x - (w if align == 2 else (w * 0.5 if align == 1 else 0.0))
	draw_string(ff, Vector2(x, p.y), s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, col)
	return w

## The game's own painted title (cut from its sprites), centred at c; false if there is none.
var title_tex: Texture2D

func _title(c: Vector2, h: float) -> bool:
	if title_tex == null:
		return false
	var k := h / title_tex.get_height()
	var sz := Vector2(title_tex.get_width(), title_tex.get_height()) * k
	draw_texture_rect(title_tex, Rect2(c - sz * 0.5, sz), false)
	return true

func _draw() -> void:
	buttons.clear()
	if data.is_empty() or font == null:
		return
	var st := _stage()
	draw_set_transform(st[1], 0.0, Vector2(st[0], st[0]))
	var tab := Rect2(60, 26, 620, 90)
	Ui.draw(self, "tab", tab, tab.size.y / 25.0)
	if not _title(tab.get_center() + Vector2(20, 0), 72.0):
		_txt(Vector2(tab.get_center().x, tab.position.y + 64), "Configuration", 50, INK, title_font, false, 1)
	var rows: Array = data.rows
	var cur_row: int = int(data.row)
	var panel := Rect2(60, 140, 1480, 600)
	Ui.draw(self, "win", panel, U)
	var n := rows.size()
	var rh := minf(44.0, (panel.size.y - 30) / maxi(n, 1))
	for i in n:
		var r: Dictionary = rows[i]
		var y := panel.position.y + 18 + rh * i
		var rr := Rect2(panel.position.x + 14, y, panel.size.x - 28, rh - 2)
		if i == cur_row:
			Ui.bar(self, rr, U)
		_txt(Vector2(rr.position.x + 24, y + rh * 0.72), str(r.label), int(rh * 0.68), LABEL)
		var vals: PackedStringArray = r.values
		var cur: int = int(r.cur)
		var x := rr.position.x + 560
		for v in vals.size():
			var on := v == cur
			var fs := int(rh * 0.66)
			var w := font.get_string_size(vals[v].strip_edges(), HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
			var vr := Rect2(x - 12, y + 3, w + 24, rh - 8)
			if on:
				_txt(Vector2(x, y + rh * 0.72), vals[v].strip_edges(), fs, ON)
			else:
				_txt(Vector2(x, y + rh * 0.72), vals[v].strip_edges(), fs, OFF)
			# click: go to this row, then step to this value
			var keys := []
			var dr := i - cur_row
			for k in absi(dr):
				keys.append(KDOWN if dr > 0 else KUP)
			var dv := v - cur
			for k in absi(dv):
				keys.append(KRIGHT if dv > 0 else KLEFT)
			buttons.append([vr, keys])
			x += maxf(w + 60, 170)
		var lk := []
		var d2 := i - cur_row
		for k in absi(d2):
			lk.append(KDOWN if d2 > 0 else KUP)
		buttons.append([Rect2(rr.position, Vector2(540, rr.size.y)), lk])
	# help line
	var hb := Rect2(60, 770, 1480, 100)
	Ui.draw(self, "win", hb, U)
	_txt(Vector2(hb.position.x + 40, hb.position.y + 64), str(data.help), 40, ON)
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
