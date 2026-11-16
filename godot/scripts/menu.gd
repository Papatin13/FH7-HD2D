## Menu windows (action menu, item list, map menu, use/discard ...) drawn natively
## in the style of the original: blue window with a light rim, white lettering
## with a soft shadow, item icons and remaining uses, greyed entries and the
## pointing hand. Coordinates come in frame pixels; the scale follows the parent.
extends Control
const Ui = preload("res://scripts/ui.gd")

const WIN_TOP := Color(0.33, 0.43, 0.86)
const WIN_BOTTOM := Color(0.2, 0.27, 0.66)
const RIM_LIGHT := Color(0.9, 0.93, 1.0)
const RIM_DARK := Color(0.1, 0.12, 0.3)
const TEXT := Color(0.98, 0.98, 1.0)
const TEXT_OFF := Color(0.62, 0.66, 0.8)
const USES := Color(0.75, 0.88, 1.0)
# the game's text colours: normal, grey, blue, gold, green, black
const TEXT_COLORS := [Color(0.98, 0.98, 1.0), Color(0.62, 0.66, 0.8), Color(0.55, 0.85, 1.0), Color(1.0, 0.86, 0.4), Color(0.6, 1.0, 0.6), Color(0.15, 0.1, 0.05)]

var font: Font
var ink_font: Font
var menus: Array = []
var help := {}
var fc := {}
const RED_TOP := Color(0.78, 0.22, 0.24)
const RED_BOTTOM := Color(0.55, 0.12, 0.16)
const BLUE_TOP := Color(0.25, 0.42, 0.9)
const BLUE_BOTTOM := Color(0.14, 0.26, 0.68)
const LABEL_BG := Color(0.93, 0.88, 0.68)
const LABEL_INK := Color(0.62, 0.42, 0.08)
var bg0: Image
var icon_cache := {}
var hand: Texture2D
var t := 0.0
var hand_pos := Vector2.ZERO

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_IGNORE

var generic: Array = []
var rhelp := {}
var mapexp := {}
var mapexp_t := 0.0
var help_label := "Help"
var gen_tex: Array = []

## Other game windows (pop-ups, help boxes): native frame, the game's text on it.
var gen_obj: Array = []

func set_generic(g: Array, bg: Image, objs: Array = []) -> void:
	generic = g
	gen_tex.clear()
	gen_obj.clear()
	for i in g.size():
		var src: Rect2i = g[i].src
		var img := bg.get_region(src) if bg else null
		gen_tex.append(ImageTexture.create_from_image(img) if img else null)
		var oi: Image = objs[i] if i < objs.size() else null
		gen_obj.append(ImageTexture.create_from_image(oi) if oi and not oi.is_invisible() else null)

func show_menus(m: Array, bg: Image, hand_img: Image, h := {}) -> void:
	menus = m
	help = h
	visible = not m.is_empty() or not h.is_empty() or not fc.is_empty() or not generic.is_empty() or not rhelp.is_empty() or not mapexp.is_empty()
	if not visible:
		icon_cache.clear()
		return
	bg0 = bg
	if hand_img != null and hand == null:
		hand = ImageTexture.create_from_image(hand_img)
	queue_redraw()

func _process(delta: float) -> void:
	t += delta
	if visible:
		queue_redraw()

func _scale() -> float:
	return size.y / 160.0

func _icon(gx: int, gy: int, label: String) -> Texture2D:
	var key := "%d,%d,%s" % [gx, gy, label]
	if icon_cache.has(key):
		return icon_cache[key]
	if bg0 == null or gx < 0 or gy < 0 or gx + 16 > 240 or gy + 16 > 160:
		return null
	var img := bg0.get_region(Rect2i(gx, gy, 16, 16))
	if img.is_invisible():
		return null  # not drawn yet: try again next frame
	var tex := ImageTexture.create_from_image(img)
	icon_cache[key] = tex
	return tex

func _window(r: Rect2, s: float) -> void:
	Ui.draw(self, "win", r, s)

func _text(pos: Vector2, s: float, txt: String, col: Color, right := false) -> void:
	var fs := int(round(12.0 * s))
	var p := pos
	if right:
		p.x -= font.get_string_size(txt, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	var base := p + Vector2(0, 12.0 * s)
	draw_string(font, base, txt, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, col)

func set_forecast(d: Dictionary) -> void:
	fc = d
	if not d.is_empty():
		visible = true
		queue_redraw()

func _grad(r: Rect2, top: Color, bottom: Color, radius: Array) -> void:
	var bands := 6
	for i in bands:
		var sb := StyleBoxFlat.new()
		sb.bg_color = top.lerp(bottom, float(i) / (bands - 1))
		if i == 0:
			sb.corner_radius_top_left = radius[0]
			sb.corner_radius_top_right = radius[1]
		if i == bands - 1:
			sb.corner_radius_bottom_left = radius[2]
			sb.corner_radius_bottom_right = radius[3]
		var h := r.size.y / bands
		draw_style_box(sb, Rect2(r.position.x, r.position.y + h * i, r.size.x, h + 0.6))

func _arrow(p: Vector2, s: float, up: bool) -> void:
	var c := Color(0.35, 0.95, 0.4) if up else Color(1.0, 0.35, 0.3)
	var pts := PackedVector2Array([p + Vector2(0, -4) * s, p + Vector2(4, 2) * s, p + Vector2(-4, 2) * s]) if up \
		else PackedVector2Array([p + Vector2(0, 4) * s, p + Vector2(4, -2) * s, p + Vector2(-4, -2) * s])
	draw_colored_polygon(pts, c)
	draw_polyline(PackedVector2Array([pts[0], pts[1], pts[2], pts[0]]), Color(0, 0, 0, 0.6), maxf(1.0, 0.5 * s), true)

func _badge(p: Vector2, s: float) -> void:
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.98, 0.85, 0.3)
	sb.border_color = Color(0.45, 0.25, 0.05)
	sb.set_border_width_all(int(maxf(1.0, 0.8 * s)))
	sb.set_corner_radius_all(int(3 * s))
	draw_style_box(sb, Rect2(p, Vector2(14, 10) * s))
	draw_string(ink_font if ink_font else font, p + Vector2(2.0, 8.6) * s, "×2", HORIZONTAL_ALIGNMENT_LEFT, -1, int(9 * s), Color(0.35, 0.12, 0.02))

func _num(v: int) -> String:
	return "--" if v < 0 else str(v)

func _draw_forecast(s: float) -> void:
	var rr: Rect2i = fc.rect
	var r := Rect2(Vector2(rr.position) * s, Vector2(rr.size) * s).grow(-1.0 * s)
	var p: Dictionary = fc.player
	var e: Dictionary = fc.enemy
	var rad := int(5 * s)
	# frame
	var shadow := StyleBoxFlat.new()
	shadow.bg_color = Color(0, 0, 0, 0.3)
	shadow.set_corner_radius_all(rad)
	draw_style_box(shadow, Rect2(r.position + Vector2(1.5, 2.0) * s, r.size))
	var rim := StyleBoxFlat.new()
	rim.bg_color = RIM_LIGHT
	rim.set_corner_radius_all(rad)
	draw_style_box(rim, r)
	var inner := r.grow(-1.4 * s)
	var x0 := inner.position.x
	var y0 := inner.position.y
	var w := inner.size.x
	var top_h := 22.0 * s
	var rows_h := 64.0 * s
	var bot := inner.end.y
	# player band (blue), enemy band (red)
	_grad(Rect2(x0, y0, w, top_h), BLUE_TOP, BLUE_BOTTOM, [rad - 1, rad - 1, 0, 0])
	_grad(Rect2(x0, y0 + top_h + rows_h, w, bot - (y0 + top_h + rows_h)), RED_TOP, RED_BOTTOM, [0, 0, rad - 1, rad - 1])
	# stat table: enemy column | labels | player column
	var cw := w * 0.34
	var ty := y0 + top_h
	_grad(Rect2(x0, ty, cw, rows_h), RED_TOP, RED_BOTTOM, [0, 0, 0, 0])
	draw_rect(Rect2(x0 + cw, ty, w - 2 * cw, rows_h), LABEL_BG)
	_grad(Rect2(x0 + w - cw, ty, cw, rows_h), BLUE_TOP, BLUE_BOTTOM, [0, 0, 0, 0])
	draw_rect(Rect2(x0, ty, w, 0.8 * s), Color(1, 1, 1, 0.5))
	draw_rect(Rect2(x0, ty + rows_h - 0.8 * s, w, 0.8 * s), Color(1, 1, 1, 0.5))
	var labels := ["HP", "Mt", "Hit", "Crit"]
	var ev := [e.hp, e.mt, e.hit, e.crit]
	var pv := [p.hp, p.mt, p.hit, p.crit]
	for i in 4:
		var yy := ty + (16.0 * i + 1.5) * s
		var fs := int(round(12.0 * s))
		var lw := font.get_string_size(labels[i], HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
		draw_string(ink_font if ink_font else font, Vector2(x0 + w * 0.5 - lw * 0.5, yy + 12.0 * s), labels[i], HORIZONTAL_ALIGNMENT_LEFT, -1, fs, LABEL_INK)
		_text(Vector2(x0 + cw - 4.0 * s, yy), s, _num(ev[i]), TEXT, true)
		_text(Vector2(x0 + w - 4.0 * s, yy), s, _num(pv[i]), TEXT, true)
	if e.double:
		_badge(Vector2(x0 + 1.0 * s, ty + 19.0 * s), s)
	if p.double:
		_badge(Vector2(x0 + w - cw + 1.0 * s, ty + 19.0 * s), s)
	# player header: weapon icon, triangle arrow, name
	if p.has("icon"):
		var ic := _icon(p.icon.x, p.icon.y, "fc" + str(p.weapon))
		if ic:
			draw_texture_rect(ic, Rect2(Vector2(x0 + 3.0 * s, y0 + 3.0 * s), Vector2(16, 16) * s), false)
	if p.tri != 0:
		_arrow(Vector2(x0 + 21.0 * s, y0 + 11.0 * s), s, p.tri > 0)
	_text(Vector2(x0 + 27.0 * s, y0 + 4.0 * s), s, str(p.name), TEXT)
	# enemy footer: name, weapon, icon, arrow
	var fy := ty + rows_h
	_text(Vector2(x0 + 6.0 * s, fy + 3.0 * s), s, str(e.name), TEXT)
	_text(Vector2(x0 + 6.0 * s, fy + 19.0 * s), s, str(e.weapon) if str(e.weapon) != "" else "--", TEXT)
	if e.has("icon"):
		var ic2 := _icon(e.icon.x, e.icon.y, "fc" + str(e.weapon))
		if ic2:
			draw_texture_rect(ic2, Rect2(Vector2(inner.end.x - 20.0 * s, fy + 2.0 * s), Vector2(16, 16) * s), false)
	if e.tri != 0:
		_arrow(Vector2(inner.end.x - 24.0 * s, fy + 10.0 * s), s, e.tri > 0)

func _draw() -> void:
	if font == null:
		return
	var s := _scale()
	for i in generic.size():
		var gr: Rect2i = generic[i].rect
		var r := Rect2(Vector2(gr.position) * s, Vector2(gr.size) * s).grow(-1.0 * s)
		var tabv: int = int(generic[i].get("tab", 0))
		var tabh: float = float(tabv & 0xFFFF) * s
		var tabw: float = float(tabv >> 16) * s
		if tabh > 0:
			# parchment name plate on top, window body below
			var plate := Rect2(r.position, Vector2(tabw if tabw > 0 else r.size.x, tabh))
			var pb := StyleBoxFlat.new()
			pb.bg_color = LABEL_BG
			pb.border_color = Color(0.55, 0.42, 0.22)
			pb.set_border_width_all(int(maxf(1.0, 1.2 * s)))
			pb.set_corner_radius_all(int(5 * s))
			pb.anti_aliasing = true
			draw_style_box(pb, plate)
			r = Rect2(r.position + Vector2(0, tabh), r.size - Vector2(0, tabh))
		_window(r, s)
		var texts: Array = generic[i].get("texts", [])
		var rest: Array = generic[i].get("rest", [])
		if gen_tex[i] and texts.is_empty():
			draw_texture_rect(gen_tex[i], Rect2(Vector2(gr.position) * s, Vector2(gr.size) * s), false)
		elif gen_tex[i]:
			# only what is not text (icons): copy those cells of the game's layer
			var src: Rect2i = generic[i].src
			for c in rest:
				var cp: Vector2i = c
				var local := Vector2(cp - src.position)
				draw_texture_rect_region(gen_tex[i], Rect2((Vector2(gr.position) + local) * s, Vector2(8, 8) * s), Rect2(local, Vector2(8, 8)))
		for tx in texts:
			var tp: Vector2i = tx.pos
			var tc: Color = TEXT_COLORS[clampi(int(tx.color), 0, TEXT_COLORS.size() - 1)]
			if tabh > 0 and float(tp.y) * s < r.position.y:
				# on the name plate
				draw_string(ink_font if ink_font else font, Vector2(tp) * s + Vector2(0, 13.0 * s), str(tx.text), HORIZONTAL_ALIGNMENT_LEFT, -1, int(round(12.0 * s)), LABEL_INK)
				continue
			_text(Vector2(tp) * s + Vector2(0, 1.0 * s), s, str(tx.text), tc)
		if gen_obj[i]:
			draw_texture_rect(gen_obj[i], Rect2(Vector2(gr.position) * s, Vector2(gr.size) * s), false)
	if not fc.is_empty():
		_draw_forecast(s)
	if not help.is_empty():
		var hr: Rect2i = help.rect
		var r := Rect2(Vector2(hr.position) * s, Vector2(hr.size) * s).grow(-1.0 * s)
		_window(r, s)
		if help.has("stats"):
			var st: PackedInt32Array = help.stats
			var w := r.size.x
			_text(r.position + Vector2(w * 0.5 - 24.0 * s, 7.0 * s), s, "Affi", TEXT)
			if help.has("icon"):
				var ip: Vector2i = help.icon
				var ic := _icon(ip.x, ip.y, "affi")
				if ic:
					draw_texture_rect(ic, Rect2(r.position + Vector2(w * 0.5 + 2.0 * s, 6.0 * s), Vector2(16, 16) * s), false)
			var names := ["Atk", "Hit", "Crit", "Avoid"]
			for k in 4:
				var col := k / 2
				var row := k % 2
				var x0 := r.position.x + (8.0 if col == 0 else w / s * 0.5 + 2.0) * s
				var xr := r.position.x + w * 0.5 - 6.0 * s if col == 0 else r.end.x - 8.0 * s
				var yy := r.position.y + (24.0 + 16.0 * row) * s
				_text(Vector2(x0, yy), s, names[k], TEXT)
				_text(Vector2(xr, yy), s, str(st[k]), USES, true)
		else:
			var y := 0
			for line in str(help.text).split("\n"):
				_text(r.position + Vector2(8.0, 7.0 + 16.0 * y) * s, s, line, TEXT)
				y += 1
	for m in menus:
		var rr: Rect2i = m.rect
		var r := Rect2(Vector2(rr.position) * s, Vector2(rr.size) * s).grow(-1.0 * s)
		_window(r, s)
		var items: Array = m.items
		var cur: int = m.cur
		for i in items.size():
			var it: Dictionary = items[i]
			var p := Vector2(it.pos) * s
			if i == cur and m.active:
				var hl := Rect2(Vector2(r.position.x + 4 * s, p.y + 0.5 * s), Vector2(r.size.x - 8 * s, 15 * s))
				Ui.bar(self, Rect2(Vector2(p.x, hl.position.y), Vector2(r.end.x - 5 * s - p.x, hl.size.y)), s)
			if it.icon:
				var gp: Vector2i = it.game_pos
				var ic := _icon(gp.x - 16, gp.y, str(it.label))
				if ic:
					draw_texture_rect(ic, Rect2(p - Vector2(16, 0) * s, Vector2(16, 16) * s), false)
			var col := TEXT if it.enabled else TEXT_OFF
			_text(p + Vector2(1.0 * s, 0), s, str(it.label), col)
			if int(it.uses) >= 0:
				_text(Vector2(r.end.x - 7 * s, p.y), s, str(it.uses), USES, true)
		# pointing hand next to the current entry
		if m.active and cur >= 0 and cur < items.size():
			var it: Dictionary = items[cur]
			var target := Vector2(it.pos) * s + Vector2(-14.0 - (16.0 if it.icon else 0.0), 0.0) * s
			if hand_pos.distance_to(target) > 60.0 * s:
				hand_pos = target
			hand_pos = hand_pos.lerp(target, 0.45)
			var bob := Vector2(sin(t * 6.0) * 1.2 * s, 0)
			if hand:
				draw_texture_rect(hand, Rect2(hand_pos + bob, Vector2(16, 16) * s), false)
	_draw_rhelp(s)
	_draw_mapexp(s)

func _draw_rhelp(s: float) -> void:
	if rhelp.is_empty():
		return
	var lines: PackedStringArray = rhelp.lines
	if lines.is_empty():
		return
	var fs := int(11.5 * s)
	var w := 0.0
	for l in lines:
		w = maxf(w, font.get_string_size(l, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x)
	var gr: Rect2i = rhelp.rect
	var r := Rect2(Vector2(gr.position) * s, Vector2(w + 16 * s, (21 + 14 * lines.size()) * s))
	r.position.x = clampf(r.position.x, 2 * s, size.x - r.size.x - 2 * s)
	r.position.y = clampf(r.position.y, 10 * s, size.y - r.size.y - 2 * s)
	var sh := StyleBoxFlat.new()
	sh.bg_color = Color(0, 0, 0, 0.3)
	sh.set_corner_radius_all(int(6 * s))
	draw_style_box(sh, Rect2(r.position + Vector2(1.5, 2) * s, r.size))
	var b := StyleBoxFlat.new()
	b.bg_color = Color(0.97, 0.94, 0.84)
	b.border_color = Color(0.55, 0.42, 0.22)
	b.set_border_width_all(int(maxf(1.0, 1.2 * s)))
	b.set_corner_radius_all(int(6 * s))
	b.anti_aliasing = true
	draw_style_box(b, r)
	var tw := font.get_string_size(help_label, HORIZONTAL_ALIGNMENT_LEFT, -1, int(9 * s)).x + 10 * s
	var tab := Rect2(r.position + Vector2(5 * s, 3 * s), Vector2(tw, 10 * s))
	var tb := StyleBoxFlat.new()
	tb.bg_color = LABEL_BG
	tb.border_color = Color(0.55, 0.42, 0.22)
	tb.set_border_width_all(int(maxf(1.0, 1.0 * s)))
	tb.set_corner_radius_all(int(4 * s))
	draw_style_box(tb, tab)
	draw_string(ink_font if ink_font else font, tab.position + Vector2(5 * s, 9 * s), help_label, HORIZONTAL_ALIGNMENT_LEFT, -1, int(9 * s), LABEL_INK)
	for i in lines.size():
		draw_string(ink_font if ink_font else font, r.position + Vector2(8 * s, (26 + 14 * i) * s), lines[i], HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Color(0.2, 0.14, 0.08))

func _draw_mapexp(s: float) -> void:
	if mapexp.is_empty():
		mapexp_t = 0.0
		return
	mapexp_t += get_process_delta_time()
	var gr: Rect2i = mapexp.rect
	var r := Rect2(Vector2(gr.position) * s, Vector2(gr.size) * s).grow(-1.0 * s)
	_window(r, s)
	var old: int = mapexp.old
	var new: int = mapexp.new
	var v := lerpf(old, new if new >= old else new + 100, clampf(mapexp_t / 1.2, 0.0, 1.0))
	if v >= 100.0:
		v -= 100.0
	_text(r.position + Vector2(6, 2) * s, s, "EXP", Color(1.0, 0.86, 0.4))
	var bar := Rect2(r.position + Vector2(32, 6) * s, Vector2(r.size.x / s - 64, 7) * s)
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.05, 0.07, 0.2)
	sb.set_corner_radius_all(int(3 * s))
	draw_style_box(sb, bar)
	var fill := StyleBoxFlat.new()
	fill.bg_color = Color(1.0, 0.85, 0.3)
	fill.set_corner_radius_all(int(3 * s))
	draw_style_box(fill, Rect2(bar.position, Vector2(bar.size.x * v / 100.0, bar.size.y)))
	_text(Vector2(r.end.x - 6 * s, r.position.y + 2 * s), s, str(int(v)), TEXT, true)
