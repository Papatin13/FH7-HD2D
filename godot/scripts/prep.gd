## Preparations screen before a chapter, drawn natively: chapter and goal on a
## parchment banner, the commands in a blue panel, the help text beside it and
## a "Fight!" button (the game's START). Clicking a command moves the game's
## cursor there and confirms it.
extends Control
const Ui = preload("res://scripts/ui.gd")

signal key(gba: int)

const W := 1600.0
const H := 900.0
const U := 5.625   # one original pixel on the 1600x900 stage
const FS := 1.3    # the game font runs smaller than the old one
const PANEL_TOP := Color(0.3, 0.38, 0.85)
const PANEL_BOTTOM := Color(0.18, 0.24, 0.66)
const RIM_LIGHT := Color(0.9, 0.93, 1.0)
const RIM_DARK := Color(0.1, 0.12, 0.3)
const ON := Color(1, 1, 1)
const OFF := Color(0.6, 0.64, 0.82)
const GOLDC := Color(1.0, 0.86, 0.35)
const SHADOW := Color(0.04, 0.05, 0.18, 0.85)
const PARCH := Color(0.94, 0.89, 0.72)
const PARCH_DARK := Color(0.66, 0.56, 0.34)
const INK := Color(0.28, 0.18, 0.08)
const KA := 1
const KB := 2
const KSTART := 8
const KUP := 64
const KDOWN := 128

var font: Font
var title_font: Font
var ink_font: Font     # dialogue font: dark text on light boxes
var data := {}
var t := 0.0
var buttons := []
var hover := -1
var labels := {}   # translated captions: "Preparations", "Fight!", "Goal", "Funds"

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_STOP

var target := -1       # row clicked: the cursor is walked there, then confirmed
var step_wait := 0

func show_prep(d: Dictionary) -> void:
	data = d
	visible = not d.is_empty()
	if not visible:
		target = -1
	elif target >= 0:
		step_wait -= 1
		if step_wait <= 0:
			var cur := int(d.cursor)
			if cur == target:
				var it: Dictionary = (d.items as Array)[target] if target < (d.items as Array).size() else {}
				if not it.is_empty() and not bool(it.off):
					key.emit(KA)
				target = -1
			else:
				key.emit(KDOWN if target > cur else KUP)
			step_wait = 8
	if visible:
		queue_redraw()

func _process(delta: float) -> void:
	t += delta
	if visible:
		queue_redraw()

func _stage() -> Array:
	var k := minf(size.x / W, size.y / H)
	return [k, (size - Vector2(W, H) * k) * 0.5]

func _L(k: String) -> String:
	return str(labels.get(k, k))

func _gui_input(ev: InputEvent) -> void:
	var st := _stage()
	if ev is InputEventMouseMotion:
		var p: Vector2 = (ev.position - st[1]) / st[0]
		hover = -1
		for i in buttons.size():
			if (buttons[i][0] as Rect2).has_point(p):
				hover = i
		# hovering a command moves the game's cursor (its help text follows)
		if hover >= 0 and buttons[hover].size() > 2 and data.size() > 0:
			var row: int = buttons[hover][2]
			var cur: int = int(data.cursor)
			if row != cur and not _moving and target < 0:
				_moving = true
				var d := row - cur
				for k in absi(d):
					key.emit(KDOWN if d > 0 else KUP)
				get_tree().create_timer(0.12).timeout.connect(func(): _moving = false)
		return
	if not (ev is InputEventMouseButton and ev.pressed):
		return
	var p: Vector2 = (ev.position - st[1]) / st[0]
	match ev.button_index:
		MOUSE_BUTTON_LEFT:
			for b in buttons:
				if (b[0] as Rect2).has_point(p):
					if b.size() > 2:
						target = b[2]
						step_wait = 0
					else:
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

var _moving := false

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

func _panel(r: Rect2) -> void:
	Ui.draw(self, "win", r, U)

func _parch(r: Rect2, rad := 24) -> void:
	if rad >= 40:
		Ui.draw(self, "tab", r, r.size.y / 25.0)
	else:
		Ui.draw(self, "parch", r, U)

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
	# banner: "Preparations" and the chapter with its goal
	var tab := Rect2(60, 26, 560, 96)
	_parch(tab, 45)
	if not _title(tab.get_center() + Vector2(20, 0), 70.0):
		_txt(Vector2(tab.get_center().x, tab.position.y + 68), _L("Preparations"), 52, INK, title_font, false, 1)
	var cb := Rect2(660, 26, 880, 96)
	_parch(cb, 24)
	_txt(Vector2(cb.position.x + 34, cb.position.y + 50), str(data.title), 30, INK, null, false)
	_txt(Vector2(cb.position.x + 34, cb.position.y + 84), _L("Goal") + ": " + str(data.goal), 26, Color(0.45, 0.3, 0.15), font, false)
	# commands
	var items: Array = data.items
	var cur: int = int(data.cursor)
	var panel := Rect2(60, 160, 560, 520)
	_panel(panel)
	var rh := 92.0
	for i in items.size():
		var it: Dictionary = items[i]
		var rr := Rect2(panel.position.x + 22, panel.position.y + 26 + rh * i, panel.size.x - 44, rh - 10)
		if i == cur:
			Ui.bar(self, rr, U)
		var off: bool = it.off
		_txt(Vector2(rr.position.x + 36, rr.position.y + rh * 0.62), str(it.label), 46, OFF if off else ON)
		var keys := []
		var d := i - cur
		for k in absi(d):
			keys.append(KDOWN if d > 0 else KUP)
		if not off:
			keys.append(KA)
		buttons.append([rr, keys, i])
	# help
	var hb := Rect2(660, 160, 880, 520)
	Ui.draw(self, "prephelp", hb, U)
	var lines: PackedStringArray = data.help
	for i in lines.size():
		_txt(Vector2(hb.position.x + 70, hb.position.y + 100 + 72 * i), lines[i], 44, ON)
	_txt(Vector2(hb.get_center().x, hb.end.y - 40), _L("Command Help"), 40, INK, null, false, 1)
	# funds + fight
	var fb := Rect2(60, 712, 560, 130)
	_panel(fb)
	_txt(Vector2(fb.position.x + 40, fb.position.y + 84), _L("Funds"), 44, GOLDC)
	_txt(Vector2(fb.end.x - 40, fb.position.y + 84), "%d G" % int(data.gold), 48, ON, null, true, 2)
	var fight := Rect2(1080, 712, 460, 130)
	Ui.draw(self, "bar", fight, fight.size.y / 23.0)
	_txt(Vector2(fight.get_center().x, fight.position.y + 88), _L("Fight!"), 50, ON, null, true, 1)
	buttons.append([fight, [KSTART]])
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
