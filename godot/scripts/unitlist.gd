## Unit list (the army overview from the map menu), drawn natively on a 1600x900
## stage in the style of the original: page tab with arrows, sort box, a blue
## table with a parchment header row and the units' map sprites. The game keeps
## the logic (sorting, cursor, paging); clicks are turned into its keys.
extends Control
const Ui = preload("res://scripts/ui.gd")

signal key(gba: int)

const W := 1600.0
const H := 900.0
const U := 5.625   # one original pixel on the 1600x900 stage
const FS := 1.3    # the game font runs smaller than the old one
const ROWS := 7
const PANEL_TOP := Color(0.3, 0.38, 0.85)
const PANEL_BOTTOM := Color(0.18, 0.24, 0.66)
const RIM_LIGHT := Color(0.9, 0.93, 1.0)
const RIM_DARK := Color(0.1, 0.12, 0.3)
const LABEL := Color(1.0, 0.97, 0.45)
const VALUE := Color(0.75, 0.94, 1.0)
const TEXT := Color(0.98, 0.98, 1.0)
const SHADOW := Color(0.04, 0.05, 0.18, 0.85)
const PARCH := Color(0.94, 0.89, 0.72)
const PARCH_DARK := Color(0.66, 0.56, 0.34)
const INK := Color(0.28, 0.18, 0.08)
const AFFINITY := ["---", "Fire", "Thunder", "Wind", "Ice", "Dark", "Light", "Anima"]
const RANKS := ["Sword", "Lance", "Axe", "Bow", "Staff", "Anima", "Light", "Dark"]
const KA := 1
const KB := 2
const KRIGHT := 16
const KLEFT := 32
const KUP := 64
const KDOWN := 128

# per page: title, columns [label, key, width, align(0 left / 2 right)]
var PAGES := [
	["Character", [["Class", "class", 300, 0], ["Lv", "lv", 110, 2], ["Exp", "exp", 110, 2], ["HP", "hp", 120, 2], ["Max", "max", 120, 2]]],
	["Fighting Skill", [["S/M", "s0", 95, 2], ["Skill", "s1", 95, 2], ["Spd", "s2", 95, 2], ["Luck", "s3", 95, 2], ["Def", "s4", 95, 2], ["Res", "s5", 95, 2], ["Affin", "aff", 150, 2]]],
	["Equipment", [["Equip", "equip", 340, 0], ["Atk", "atk", 120, 2], ["Hit", "hit", 120, 2], ["Avoid", "avo", 140, 2]]],
	["Personal Data", [["Move", "move", 110, 2], ["Con", "con", 110, 2], ["Aid", "aid", 110, 2], ["Trv", "trv", 200, 2], ["Cond", "cond", 190, 2]]],
	["Weapon Level", [["Swd", "r0", 85, 2], ["Lnc", "r1", 85, 2], ["Axe", "r2", 85, 2], ["Bow", "r3", 85, 2], ["Stf", "r4", 85, 2], ["Anm", "r5", 85, 2], ["Lgt", "r6", 85, 2], ["Drk", "r7", 85, 2]]],
	["Support Chance", [["Ally", "allies", 720, 0]]],
]

var font: Font
var title_font: Font
var ink_font: Font     # dialogue font: dark text on light boxes
var data := {}
var icons := {}          # row -> Texture2D
var t := 0.0
var buttons := []        # [Rect2, Array of keys]

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_STOP

func show_list(d: Dictionary, row_icons: Dictionary) -> void:
	data = d
	visible = not d.is_empty()
	if row_icons.size() > 0:
		icons = row_icons
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

func _txt(p: Vector2, s: String, fs: int, col: Color, f: Font = null, shadow := true, align := 0, maxw := 0.0) -> void:
	var ff: Font = f if f else font
	if (f == null or f == font) and ink_font and col.v < 0.5:
		ff = ink_font
	if ff != title_font:
		fs = int(fs * FS)
	var w := ff.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	if maxw > 0.0 and w > maxw:
		fs = maxi(8, int(fs * maxw / w))
		w = ff.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	var x := p.x - (w if align == 2 else (w * 0.5 if align == 1 else 0.0))
	draw_string(ff, Vector2(x, p.y), s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, col)

func _arrow(c: Vector2, left: bool, k: int) -> void:
	var bob := sin(t * 5.0) * 4.0 * (-1.0 if left else 1.0)
	var d := -1.0 if left else 1.0
	var pts := PackedVector2Array([c + Vector2(-12 * d + bob, -20), c + Vector2(14 * d + bob, 0), c + Vector2(-12 * d + bob, 20)])
	draw_colored_polygon(pts, Color(1.0, 0.86, 0.35))
	draw_polyline(PackedVector2Array([pts[0], pts[1], pts[2], pts[0]]), Color(0.45, 0.28, 0.05), 3.0, true)
	buttons.append([Rect2(c - Vector2(34, 34), Vector2(68, 68)), [k]])

func _sort_label(id: int) -> String:
	var n := 1
	if id == 1:
		return "Name"
	for pg in PAGES:
		for c in pg[1]:
			n += 1
			if n == id:
				return c[0]
	return "Name"

func _value(u: Dictionary, k: String) -> String:
	if k.begins_with("s") and k.length() == 2:
		var v: PackedInt32Array = u.stats
		return str(v[int(k.substr(1))])
	if k.begins_with("r") and k.length() == 2:
		var r: PackedStringArray = u.ranks
		return r[int(k.substr(1))]
	match k:
		"aff":
			return AFFINITY[clampi(int(u.affinity), 0, 7)]
		"equip":
			return str(u.equip) if str(u.equip) != "" else "-----"
		"atk", "hit":
			return "--" if int(u[k]) < 0 else str(u[k])
		"trv", "cond":
			return str(u[k]) if str(u[k]) != "" else "---"
		"allies":
			var a: PackedStringArray = u.allies
			return "   ".join(a) if a.size() > 0 else "---"
	return str(u.get(k, ""))

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
	var page: int = clampi(int(data.page), 0, 5)
	var pg: Array = PAGES[page]
	# title tab
	var tab := Rect2(60, 34, 760, 96)
	Ui.draw(self, "tab", tab, tab.size.y / 25.0)
	if not _title(tab.get_center(), 76.0):
		_txt(Vector2(tab.get_center().x, tab.position.y + 68), pg[0], 52, INK, title_font, false, 1)
	_arrow(Vector2(tab.position.x + 150, tab.get_center().y), true, KLEFT)
	_arrow(Vector2(tab.end.x - 52, tab.get_center().y), false, KRIGHT)
	# sort box
	var sb := Rect2(860, 34, 420, 96)
	Ui.draw(self, "win", sb, U)
	_txt(Vector2(sb.position.x + 26, sb.position.y + 40), "Sort", 30, LABEL)
	var desc: bool = int(data.desc) == 1
	_txt(Vector2(sb.position.x + 26, sb.position.y + 82), _sort_label(int(data.sort)) + ("  ▼" if desc else "  ▲"), 40, TEXT)
	# page counter
	var pc := Rect2(1320, 34, 220, 96)
	Ui.draw(self, "win", pc, U)
	_txt(Vector2(pc.get_center().x, pc.position.y + 66), "%d / 6" % (page + 1), 46, TEXT, null, true, 1)
	# table
	var panel := Rect2(60, 160, 1480, 700)
	Ui.draw(self, "win", panel, U)
	var head := Rect2(panel.position + Vector2(14, 14), Vector2(panel.size.x - 28, 70))
	_box(head, PARCH, 10)
	# column x positions
	var cols: Array = [["Name", "name", 330, 0]] + pg[1]
	var xs := []
	var x := head.position.x + 110
	for c in cols:
		xs.append(x)
		x += float(c[2]) + 20.0
	var header_mode := int(data.mode) == 3
	var hcol: int = int(data.hcol)
	var sort_id: int = int(data.sort)
	var first_id := 2
	for i in page:
		first_id += (PAGES[i][1] as Array).size()
	for i in cols.size():
		var c: Array = cols[i]
		var cx: float = xs[i]
		var cw: float = c[2]
		var r := Rect2(cx - 10, head.position.y + 6, cw + 20, head.size.y - 12)
		var cid := 1 if i == 0 else first_id + i - 1
		if header_mode and i == hcol:
			_box(r, Color(1.0, 0.82, 0.4, 0.75 + 0.2 * sin(t * 6.0)), 8)
		var lab: String = c[0]
		if cid == sort_id:
			lab += " ▼" if desc else " ▲"
		var ax: float = cx if int(c[3]) == 0 else cx + cw
		_txt(Vector2(ax, head.position.y + 50), lab, 36, INK, null, false, int(c[3]))
		# click: go to the header and sort by this column
		var keys := []
		if not header_mode:
			for k in int(data.row) + 1:
				keys.append(KUP)
		var dcol := i - hcol
		for k in absi(dcol):
			keys.append(KRIGHT if dcol > 0 else KLEFT)
		keys.append(KA)
		buttons.append([r, keys])
	# rows
	var units: Array = data.units
	var sel: int = int(data.sel)
	var vrow: int = int(data.row)
	var top := sel - vrow
	var y0 := head.end.y + 18
	var rh := 84.0
	for vi in ROWS:
		var idx := top + vi
		if idx < 0 or idx >= units.size():
			continue
		var u: Dictionary = units[idx]
		var ry := y0 + rh * vi
		var rr := Rect2(head.position.x, ry, head.size.x, rh - 8)
		if idx == sel and not header_mode:
			Ui.bar(self, rr, U)
		var ic: Texture2D = icons.get(vi)
		if ic:
			draw_texture_rect(ic, Rect2(Vector2(head.position.x + 20, ry + 2), Vector2(72, 72)), false)
		for i in cols.size():
			var c: Array = cols[i]
			var v := _value(u, c[1])
			var cw: float = c[2]
			var ax: float = xs[i] if int(c[3]) == 0 else xs[i] + cw
			var col := TEXT if i == 0 or c[1] in ["class", "equip", "trv", "cond", "allies", "aff"] else VALUE
			_txt(Vector2(ax, ry + 52), v, 40, col, null, true, int(c[3]), cw)
		# click: move the cursor there, a second click opens the unit
		var keys := []
		if header_mode:
			keys.append(KDOWN)
			var dh := vi - vrow
			for k in absi(dh):
				keys.append(KDOWN if dh > 0 else KUP)
		elif idx == sel:
			keys.append(KA)
		else:
			var dd := vi - vrow
			for k in absi(dd):
				keys.append(KDOWN if dd > 0 else KUP)
		buttons.append([rr, keys])
	# scroll hints
	if top > 0:
		_txt(Vector2(panel.get_center().x, head.end.y + 14), "▲", 30, LABEL, null, true, 1)
	if top + ROWS < units.size():
		_txt(Vector2(panel.get_center().x, panel.end.y - 8), "▼", 30, LABEL, null, true, 1)
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
