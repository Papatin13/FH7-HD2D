## Unit status screen, drawn natively in the style of the original (green
## backdrop, portrait card on parchment, blue data panel with three pages:
## personal data, items, weapon and support level). The game keeps running
## underneath; this screen only shows its data. Drawn on a 1600x900 stage.
extends Control
const Ui = preload("res://scripts/ui.gd")

signal key(gba: int)   # page / unit buttons -> GBA keys

const W := 1600.0
const H := 900.0
const U := 5.625   # one original pixel on the 1600x900 stage
const FS := 1.3    # the game font runs smaller than the old one
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
const INK := Color(0.24, 0.15, 0.08)
const AFFINITY := ["", "Fire", "Thunder", "Wind", "Ice", "Dark", "Light", "Anima"]
const AFF_COL := [Color.WHITE, Color(0.95, 0.35, 0.2), Color(1.0, 0.85, 0.2), Color(0.4, 0.85, 0.4),
	Color(0.55, 0.85, 1.0), Color(0.45, 0.3, 0.65), Color(1.0, 0.97, 0.75), Color(0.85, 0.6, 0.95)]
const TITLES := ["Personal Data", "Items", "Weapon & Support Level"]
const RANK_NAMES := ["Sword", "Lance", "Axe", "Bow", "Staff", "Anima", "Light", "Dark"]

var font: Font         # lettering (narrow)
var title_font: Font   # headings (Fell)
var ink_font: Font
var data := {}
var frame: Image
var portrait: ImageTexture
var icons := {}        # "name|page|slot" -> Texture2D
var stable := 0
var last_key := ""
var t := 0.0
var buttons := []      # [Rect2 (stage), gba key]

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_STOP

func show_status(d: Dictionary, frm: Image) -> void:
	data = d
	frame = frm
	visible = not d.is_empty()
	if not visible:
		return
	var k := "%s|%d" % [d.name, d.page]
	stable = stable + 1 if k == last_key else 0
	last_key = k
	var ext: int = d.get("ext", 0)
	if frame != null:
		# portrait from the game's picture (80x72 at 16,8)
		var img := frame.get_region(Rect2i(16 + ext, 8, 80, 72))
		if portrait == null:
			portrait = ImageTexture.create_from_image(img)
		else:
			portrait.update(img)
		# icons of the current page once its slide animation is over
		if stable == 24:
			_grab_icons(ext)
	queue_redraw()

func _grab_icons(ext: int) -> void:
	var page: int = data.page
	var spots := []
	if page == 0:
		spots = [[200, 88]]
	elif page == 1:
		for i in data.items.size():
			spots.append([104, 24 + 16 * i])
	else:
		spots = [[104, 24], [168, 24], [104, 40], [168, 40]]
	for i in spots.size():
		var img := frame.get_region(Rect2i(spots[i][0] + ext, spots[i][1], 16, 16))
		icons["%s|%d|%d" % [data.name, page, i]] = ImageTexture.create_from_image(img)

func _icon(page: int, slot: int) -> Texture2D:
	return icons.get("%s|%d|%d" % [data.name, page, slot])

func _process(delta: float) -> void:
	t += delta
	if visible:
		queue_redraw()

# ---------------------------------------------------------------- stage helpers

func _stage() -> Array:
	var k := minf(size.x / W, size.y / H)
	var off := (size - Vector2(W, H) * k) * 0.5
	return [k, off]

func _gui_input(ev: InputEvent) -> void:
	if ev is InputEventMouseButton and ev.pressed:
		var st := _stage()
		var p: Vector2 = (ev.position - st[1]) / st[0]
		match ev.button_index:
			MOUSE_BUTTON_LEFT:
				for b in buttons:
					if (b[0] as Rect2).has_point(p):
						key.emit(b[1])
						accept_event()
						return
			MOUSE_BUTTON_RIGHT:
				key.emit(2)   # B: back
				accept_event()
			MOUSE_BUTTON_WHEEL_UP:
				key.emit(64)
				accept_event()
			MOUSE_BUTTON_WHEEL_DOWN:
				key.emit(128)
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

func _bar(r: Rect2, frac: float, col: Color) -> void:
	_box(r, Color(0.06, 0.08, 0.2, 0.9), int(r.size.y * 0.5))
	if frac > 0.0:
		_box(Rect2(r.position + Vector2(3, 3), Vector2((r.size.x - 6) * clampf(frac, 0.0, 1.0), r.size.y - 6)), col, int(r.size.y * 0.5) - 2)

func _arrow(c: Vector2, left: bool, k: int) -> void:
	var bob := sin(t * 5.0) * 4.0 * (-1.0 if left else 1.0)
	var d := -1.0 if left else 1.0
	var pts := PackedVector2Array([c + Vector2(-12 * d + bob, -20), c + Vector2(14 * d + bob, 0), c + Vector2(-12 * d + bob, 20)])
	draw_colored_polygon(pts, Color(1.0, 0.86, 0.35))
	draw_polyline(PackedVector2Array([pts[0], pts[1], pts[2], pts[0]]), Color(0.45, 0.28, 0.05), 3.0, true)
	buttons.append([Rect2(c - Vector2(30, 30), Vector2(60, 60)), k])

# ---------------------------------------------------------------- drawing

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
	_draw_left()
	_draw_right()
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
	# fade with the game
	var f: int = data.get("fade", 0)
	if f != 0:
		draw_rect(Rect2(Vector2.ZERO, size), Color(0, 0, 0, -f / 16.0) if f < 0 else Color(1, 1, 1, f / 16.0))

func _draw_left() -> void:
	# portrait card on parchment
	var card := Rect2(70, 50, 520, 520)
	Ui.draw(self, "parch", card, U)
	var pic := Rect2(card.position + Vector2(40, 34), Vector2(440, 396))
	_box(pic.grow(6), PARCH_DARK, 6)
	_box(pic, Color(0.55, 0.62, 0.78), 2)
	if portrait:
		draw_texture_rect(portrait, pic, false)
	# name plate
	_txt(Vector2(card.get_center().x, card.end.y - 22), str(data.name), 40, INK, null, false, 1)
	# up/down: switch unit
	_arrow_v(Vector2(card.end.x + 34, card.position.y + 150), true)
	_arrow_v(Vector2(card.end.x + 34, card.position.y + 370), false)
	# class, level, exp, HP
	var info := Rect2(70, 600, 520, 250)
	_box(info, Color(0.02, 0.08, 0.04, 0.55), 14)
	_txt(Vector2(100, 660), str(data["class"]), 46, TEXT)
	_txt(Vector2(100, 740), "LV", 44, LABEL)
	_txt(Vector2(250, 740), str(data.level), 50, VALUE, null, true, 2)
	_txt(Vector2(300, 740), "E", 44, LABEL)
	var ex: int = data.exp
	_txt(Vector2(430, 740), "--" if ex == 255 else str(ex), 50, VALUE, null, true, 2)
	_txt(Vector2(100, 818), "HP", 44, LABEL)
	var hp: int = data.hp
	var mx: int = maxi(1, data.max_hp)
	_txt(Vector2(330, 818), "%d/%d" % [hp, mx], 50, VALUE, null, true, 2)
	_bar(Rect2(350, 790, 210, 24), float(hp) / mx, Color(1.0, 0.86, 0.3) if hp * 3 > mx else Color(1.0, 0.45, 0.3))

func _arrow_v(c: Vector2, up: bool) -> void:
	var d := -1.0 if up else 1.0
	var bob := sin(t * 5.0) * 3.0 * d
	var pts := PackedVector2Array([c + Vector2(-18, 10 * -d + bob), c + Vector2(18, 10 * -d + bob), c + Vector2(0, 14 * d + bob)])
	draw_colored_polygon(pts, Color(1.0, 0.86, 0.35, 0.85))
	buttons.append([Rect2(c - Vector2(30, 30), Vector2(60, 60)), 64 if up else 128])

func _draw_right() -> void:
	var page: int = clampi(data.page, 0, 2)
	# header tab with page arrows
	var tab := Rect2(660, 40, 870, 92)
	# like the original: the painted page title stands free on the backdrop
	if not _title(tab.get_center(), 84.0):
		Ui.draw(self, "tab", tab, tab.size.y / 25.0)
		_txt(Vector2(tab.get_center().x, tab.position.y + 66), TITLES[page], 50, INK, title_font, false, 1)
	_arrow(Vector2(tab.position.x + 150, tab.get_center().y), true, 32)
	_arrow(Vector2(tab.end.x - 50, tab.get_center().y), false, 16)
	# page dots
	for i in 3:
		var c := Vector2(tab.get_center().x - 30 + 30 * i, tab.end.y + 24)
		draw_circle(c, 9.0 if i == page else 7.0, Color(1.0, 0.86, 0.35) if i == page else Color(1, 1, 1, 0.45))
		buttons.append([Rect2(c - Vector2(15, 15), Vector2(30, 30)), 16 if i > page else (32 if i < page else 0)])
	var panel := Rect2(660, 170, 870, 680)
	_panel(panel)
	match page:
		0: _page_personal(panel)
		1: _page_items(panel)
		2: _page_ranks(panel)

func _page_personal(p: Rect2) -> void:
	var stats: Array = data.stats
	var x0 := p.position.x + 50
	for i in stats.size():
		var y := p.position.y + 90 + 92 * i
		var s: Array = stats[i]
		_txt(Vector2(x0, y), str(s[0]), 46, LABEL)
		_txt(Vector2(x0 + 230, y), str(s[1]), 50, VALUE, null, true, 2)
		var cap := maxi(1, int(s[2]))
		_bar(Rect2(x0 + 250, y - 26, 150, 20), float(s[1]) / cap, Color(0.55, 0.85, 1.0) if int(s[1]) < cap else Color(0.4, 1.0, 0.55))
	var x1 := p.position.x + 500
	var rows := [["Move", str(data.move)], ["Con", str(data.con)], ["Aid", str(data.aid)],
		["Trv", str(data.trv) if str(data.trv) != "" else "---"], ["Affin", ""], ["Cond", str(data.cond) if str(data.cond) != "" else "---"]]
	for i in rows.size():
		var y := p.position.y + 90 + 92 * i
		_txt(Vector2(x1, y), rows[i][0], 46, LABEL)
		if rows[i][0] == "Affin":
			var a: int = clampi(data.affinity, 0, 7)
			var ic := _icon(0, 0)
			if ic:
				draw_texture_rect(ic, Rect2(Vector2(x1 + 150, y - 46), Vector2(56, 56)), false)
			else:
				draw_circle(Vector2(x1 + 178, y - 18), 22, AFF_COL[a])
			_txt(Vector2(x1 + 220, y), AFFINITY[a], 40, VALUE)
		else:
			_txt(Vector2(x1 + 330, y), rows[i][1], 50 if rows[i][1].length() < 5 else 40, VALUE, null, true, 2)

func _page_items(p: Rect2) -> void:
	var items: Array = data.items
	var x0 := p.position.x + 50
	for i in 5:
		var y := p.position.y + 84 + 70 * i
		draw_rect(Rect2(x0 - 10, y + 16, p.size.x - 80, 2), Color(1, 1, 1, 0.12))
		if i >= items.size():
			continue
		var it: Dictionary = items[i]
		var ic := _icon(1, i)
		if ic:
			draw_texture_rect(ic, Rect2(Vector2(x0, y - 46), Vector2(56, 56)), false)
		_txt(Vector2(x0 + 76, y), str(it.name), 46, TEXT)
		var uses: int = it.uses
		var u := "--" if uses < 0 else "%d/%d" % [uses, it.max]
		_txt(Vector2(p.end.x - 120, y), u, 46, VALUE, null, true, 2)
		if it.equipped:
			_box(Rect2(p.end.x - 104, y - 40, 50, 46), Color(1.0, 0.86, 0.35), 10, Color(0.45, 0.28, 0.05), 3)
			_txt(Vector2(p.end.x - 79, y - 6), "E", 38, INK, null, false, 1)
	# equipment box
	var eb := Rect2(p.position.x + 40, p.end.y - 250, p.size.x - 80, 210)
	_box(eb, Color(0.62, 0.72, 1.0, 0.35), 14, Color(0.85, 0.9, 1.0, 0.6), 3)
	_txt(Vector2(eb.position.x + 30, eb.position.y + 62), "Equipment", 44, TEXT, title_font)
	var eq: PackedInt32Array = data.equip
	var cells := [["Rng", str(data.rng)], ["Atk", eq[0]], ["Crit", eq[2]], ["Hit", eq[1]], ["Avoid", eq[3]]]
	var pos := [Vector2(430, 62), Vector2(30, 130), Vector2(430, 130), Vector2(30, 190), Vector2(430, 190)]
	for i in cells.size():
		var b: Vector2 = eb.position + pos[i]
		_txt(b, cells[i][0], 42, LABEL)
		var v = cells[i][1]
		var vs := str(v) if v is String else ("--" if int(v) < 0 else str(v))
		_txt(b + Vector2(330, 0), vs, 46, VALUE, null, true, 2)

func _page_ranks(p: Rect2) -> void:
	var ranks: Array = data.ranks
	for i in ranks.size():
		var col := i % 2
		var row := i / 2
		var x := p.position.x + 50 + 410 * col
		var y := p.position.y + 100 + 96 * row
		var rk: Dictionary = ranks[i]
		var ic := _icon(2, i)
		if ic:
			draw_texture_rect(ic, Rect2(Vector2(x, y - 50), Vector2(60, 60)), false)
		_txt(Vector2(x + 76, y - 4), RANK_NAMES[int(rk.type)], 30, LABEL)
		_txt(Vector2(x + 330, y), str(rk.letter), 52, VALUE, null, true, 2)
		_bar(Rect2(x + 76, y + 6, 280, 16), float(rk.frac) if str(rk.letter) != "-" else 0.0, Color(0.55, 0.85, 1.0))
	var sy := p.position.y + 300
	draw_rect(Rect2(p.position.x + 40, sy - 30, p.size.x - 80, 3), Color(1, 1, 1, 0.25))
	_txt(Vector2(p.position.x + 50, sy + 30), "Support", 42, TEXT, title_font)
	var sups: Array = data.supports
	if sups.is_empty():
		_txt(Vector2(p.position.x + 50, sy + 100), "---", 44, VALUE)
	for i in sups.size():
		var sx := p.position.x + 50 + 410 * (i % 2)
		var yy := sy + 100 + 70 * (i / 2)
		_txt(Vector2(sx, yy), str(sups[i].name), 44, TEXT)
		_txt(Vector2(sx + 330, yy), str(sups[i].level), 48, VALUE, null, true, 2)
