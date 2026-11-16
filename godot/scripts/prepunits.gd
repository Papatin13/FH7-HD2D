## Preparations sub-screens drawn natively: "Pick Units" (who goes into battle)
## and "Trade" (choosing two units). The unit cards are laid out in the same
## grid as the game's own list, so a click is turned into the cursor moves plus
## a confirm, and the keyboard keeps working as before.
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
const OFF := Color(0.62, 0.66, 0.84)
const VALUE := Color(0.75, 0.94, 1.0)
const GOLDC := Color(1.0, 0.86, 0.35)
const SHADOW := Color(0.04, 0.05, 0.18, 0.85)
const PARCH := Color(0.94, 0.89, 0.72)
const PARCH_DARK := Color(0.66, 0.56, 0.34)
const INK := Color(0.28, 0.18, 0.08)
const KA := 1
const KB := 2
const KSELECT := 4
const KSTART := 8
const KRIGHT := 16
const KLEFT := 32
const KUP := 64
const KDOWN := 128
const KR := 256

var font: Font
var title_font: Font
var ink_font: Font     # dialogue font: dark text on light boxes
var icon_cb: Callable
var icons := {}
var data := {}
var labels := {}
var t := 0.0
var buttons := []
var hover := -1

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_STOP

func show_units(d: Dictionary) -> void:
	data = d
	visible = not d.is_empty()
	if visible:
		queue_redraw()

func _process(delta: float) -> void:
	t += delta
	if visible:
		queue_redraw()

func _L(k: String) -> String:
	return str(labels.get(k, k))

func _icon(n: int) -> Texture2D:
	if not icons.has(n) and icon_cb.is_valid():
		icons[n] = icon_cb.call(n)
	return icons.get(n)

func _stage() -> Array:
	var k := minf(size.x / W, size.y / H)
	return [k, (size - Vector2(W, H) * k) * 0.5]

func _cols() -> int:
	return 2 if str(data.get("mode", "")) == "pick" else 3

## Key presses that move the game's cursor from one list index to another.
func _moves(from: int, to: int) -> Array:
	var c := _cols()
	var out := []
	var dr := to / c - from / c
	var dc := to % c - from % c
	for i in absi(dr):
		out.append(KDOWN if dr > 0 else KUP)
	for i in absi(dc):
		out.append(KRIGHT if dc > 0 else KLEFT)
	return out

func _gui_input(ev: InputEvent) -> void:
	var st := _stage()
	if ev is InputEventMouseMotion:
		var p: Vector2 = (ev.position - st[1]) / st[0]
		hover = -1
		for i in buttons.size():
			if (buttons[i][0] as Rect2).has_point(p):
				hover = i
		return
	if not (ev is InputEventMouseButton and ev.pressed):
		return
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

func _panel(r: Rect2) -> void:
	Ui.draw(self, "win", r, U)

func _parch(r: Rect2, rad := 24) -> void:
	if rad >= 40:
		Ui.draw(self, "tab", r, r.size.y / 25.0)
	else:
		Ui.draw(self, "parch", r, U)

func _txt(p: Vector2, s: String, fs: int, col: Color, f: Font = null, shadow := true, align := 0) -> void:
	var ff: Font = f if f else font
	if (f == null or f == font) and ink_font and col.v < 0.5:
		ff = ink_font
	if ff != title_font:
		fs = int(fs * FS)
	var w := ff.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	var x := p.x - (w if align == 2 else (w * 0.5 if align == 1 else 0.0))
	draw_string(ff, Vector2(x, p.y), s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, col)

func _button(r: Rect2, text: String, keys: Array, red := false) -> void:
	var idx := buttons.size()
	var hot := hover == idx
	Ui.draw(self, "barsel" if (hot or red) else "bar", r, r.size.y / 23.0)
	_txt(Vector2(r.get_center().x, r.position.y + r.size.y * 0.68), text, 40, ON, null, true, 1)
	buttons.append([r, keys])

func _draw() -> void:
	buttons.clear()
	if data.is_empty() or font == null:
		return
	var st := _stage()
	draw_set_transform(st[1], 0.0, Vector2(st[0], st[0]))
	var pick := str(data.mode) == "pick"
	var units: Array = data.units
	var cur: int = int(data.cursor)
	# header
	var tab := Rect2(60, 26, 560, 96)
	_parch(tab, 45)
	_txt(Vector2(tab.get_center().x, tab.position.y + 68), _L("Pick Units" if pick else "Trade"), 52, INK, title_font, false, 1)
	var hb := Rect2(660, 26, 880, 96)
	_parch(hb)
	if pick:
		var left := int(data.max) - int(data.count)
		_txt(Vector2(hb.position.x + 34, hb.position.y + 44), _L("more") % left if left > 0 else _L("ready"), 30, INK, null, false)
		_txt(Vector2(hb.end.x - 34, hb.position.y + 62), "%d / %d" % [int(data.count), int(data.max)], 48, INK, title_font, false, 2)
	else:
		var pk: int = int(data.picked)
		var s := _L("Choose unit") if pk < 0 else _L("Trade with") % str((units[pk] as Dictionary).name)
		_txt(Vector2(hb.position.x + 34, hb.position.y + 44), s, 30, INK, null, false)
	_txt(Vector2(hb.position.x + 34, hb.position.y + 82), _L("Goal") + ": " + str(data.goal), 24, Color(0.45, 0.3, 0.15), font, false)
	# unit grid
	var cols := _cols()
	var grid := Rect2(60, 160, 900, 560)
	_panel(grid)
	var cw := (grid.size.x - 40) / cols
	var ch := 84.0
	var rows := int(ceil(units.size() / float(cols)))
	var first_row := maxi(0, cur / cols - 5)
	for i in units.size():
		var r := i / cols - first_row
		if r < 0 or r > 5:
			continue
		var u: Dictionary = units[i]
		var cell := Rect2(grid.position.x + 20 + cw * (i % cols), grid.position.y + 22 + ch * r, cw - 10, ch - 10)
		var on: bool = u.on
		if pick and on:
			_box(cell, Color(0.45, 0.65, 1.0, 0.28), 10)
		if not pick and int(data.picked) == i:
			_box(cell, Color(1.0, 0.86, 0.35, 0.4), 10)
		if i == cur:
			Ui.bar(self, cell, U)
		var col := ON if (on or not pick) else OFF
		_txt(Vector2(cell.position.x + 18, cell.position.y + 38), str(u.name), 34, col)
		_txt(Vector2(cell.position.x + 18, cell.position.y + 66), "%s  Lv %d" % [str(u["class"]), int(u.lv)], 22, VALUE if col == ON else OFF)
		_txt(Vector2(cell.end.x - 16, cell.position.y + 38), "HP %d/%d" % [int(u.hp), int(u.max)], 24, VALUE if col == ON else OFF, null, true, 2)
		if pick and bool(u.locked):
			_txt(Vector2(cell.end.x - 16, cell.position.y + 66), "★", 24, GOLDC, null, true, 2)
		elif pick and on:
			_txt(Vector2(cell.end.x - 16, cell.position.y + 66), "✔", 24, GOLDC, null, true, 2)
		var keys := _moves(cur, i)
		keys.append(KA)
		buttons.append([cell, keys])
	if rows > 6:
		_txt(Vector2(grid.end.x - 30, grid.end.y - 18), "%d/%d" % [cur / cols + 1, rows], 24, VALUE, null, true, 2)
	# items of the unit under the cursor
	var ib := Rect2(1000, 160, 540, 560)
	_panel(ib)
	if cur >= 0 and cur < units.size():
		_txt(Vector2(ib.position.x + 30, ib.position.y + 60), str((units[cur] as Dictionary).name), 42, GOLDC, title_font)
	var items: Array = data.items
	for i in items.size():
		var it: Dictionary = items[i]
		var y := ib.position.y + 100 + 72 * i
		var ic := _icon(int(it.icon))
		if ic:
			draw_texture_rect(ic, Rect2(Vector2(ib.position.x + 28, y), Vector2(48, 48)), false)
		_txt(Vector2(ib.position.x + 92, y + 38), str(it.name), 36, ON)
		var us: int = int(it.uses)
		_txt(Vector2(ib.end.x - 30, y + 38), "--" if us < 0 else str(us), 36, VALUE, null, true, 2)
	# buttons
	_button(Rect2(60, 752, 260, 100), _L("Back"), [KB])
	_button(Rect2(340, 752, 260, 100), _L("Info"), [KR])
	if pick:
		_button(Rect2(620, 752, 300, 100), _L("Unit List"), [KSELECT])
		_button(Rect2(1080, 752, 460, 100), _L("Fight!"), [KSTART], true)
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
