## Battle scene HUD, drawn natively over the game's (which it fully covers):
## name plates at the top, and per side the hit/damage/critical values, the
## weapon with its icon and the HP gauge at the bottom. Red = enemy, blue =
## own army, green = other. It slides with the game's HUD and also replaces
## the EXP gauge after the fight. Coordinates come in frame pixels.
extends Control
const Ui = preload("res://scripts/ui.gd")

const SIDE := [
	[Color(0.32, 0.46, 0.92), Color(0.13, 0.2, 0.55)],   # own
	[Color(0.3, 0.7, 0.38), Color(0.1, 0.38, 0.18)],     # other
	[Color(0.88, 0.3, 0.32), Color(0.5, 0.1, 0.14)],     # enemy
]
const RIM := Color(0.95, 0.92, 0.82)
const DARK := Color(0.08, 0.06, 0.12)
const TEXT := Color(1, 1, 1)
const SHADOW := Color(0, 0, 0, 0.7)
const LABEL := Color(1.0, 0.88, 0.45)
const PARCH := Color(0.95, 0.9, 0.76)
const INK := Color(0.25, 0.16, 0.08)

var font: Font
var ink_font: Font
var data := {}
var icons := [null, null]
var exp_t := 0.0
var exp_key := ""

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_IGNORE

var last_weapons := ""

func show_battle(d: Dictionary, icon_l: Texture2D, icon_r: Texture2D) -> void:
	data = d
	visible = not d.is_empty()
	# another fight (other weapons): the icons are fetched again
	if visible and d.has("sides"):
		var w := ""
		for sd in d.sides:
			w += str(sd.weapon) + "|"
		if w != last_weapons:
			last_weapons = w
			icons = [null, null]
	if icon_l:
		icons[0] = icon_l
	if icon_r:
		icons[1] = icon_r
	if not visible:
		icons = [null, null]
		return
	queue_redraw()

func _process(delta: float) -> void:
	if visible and data.has("exp"):
		exp_t += delta
		queue_redraw()
	elif exp_t != 0.0:
		exp_t = 0.0

func _box(r: Rect2, c: Color, rad := 4.0, s := 1.0) -> void:
	rad = 0.0
	var sb := StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(int(rad * s))
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

func _txt(p: Vector2, t: String, fs: int, col: Color, s: float, align := 0, maxw := 0.0) -> void:
	if fs < 2:
		return
	var w := font.get_string_size(t, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	if maxw > 0.0 and w > maxw:
		fs = maxi(2, int(fs * maxw / w))
		w = font.get_string_size(t, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	var x := p.x - (w if align == 2 else (w * 0.5 if align == 1 else 0.0))
	draw_string(font, Vector2(x, p.y), t, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, col)

func _draw() -> void:
	if font == null or data.is_empty() or not data.has("sides"):
		return
	var s := size.y / 160.0
	var fw := size.x / s          # frame width in game pixels
	# the HUD slides in from outside the picture; a value above it means the gauge
	# objects are not placed yet
	var hud: float = clampf(float(data.hud), 0.0, 60.0)
	var sides: Array = data.sides
	# one-sided scenes (promotion, item use): one name plate and one HUD half
	var single: bool = data.get("single", false)
	for k in 2:
		if not single or k == 1:
			_draw_plate(sides[k], k, s, fw, -hud)
		if not single or k == 0:
			_draw_bottom(sides[k], k, s, fw, hud)
	if data.has("exp"):
		_draw_exp(data.exp, s, fw, hud)

func _draw_plate(sd: Dictionary, k: int, s: float, fw: float, dy: float) -> void:
	var c: Array = SIDE[clampi(int(sd.side), 0, 2)]
	var ext: float = data.get("ext", 0)
	var w := 62.0
	var x := ext - 3.0 if k == 0 else fw - ext - w + 3.0
	var r := Rect2(Vector2(x, -3 + dy) * s, Vector2(w, 26) * s)
	# square plate with the cream rim of the original
	draw_rect(Rect2(r.position + Vector2(1, 1) * s, r.size), Color(0.2, 0.16, 0.1, 0.8))
	draw_rect(r, Color(1.0, 1.0, 0.81))
	draw_rect(r.grow(-1.0 * s), Color(0.84, 0.81, 0.58))
	_grad(r.grow(-2.0 * s), c[0], c[1], 0)
	_txt(Vector2(r.get_center().x, r.position.y + 18.5 * s), str(sd.name), int(12 * s), TEXT, s, 1, r.size.x - 8 * s)

func _draw_bottom(sd: Dictionary, k: int, s: float, fw: float, dy: float) -> void:
	var c: Array = SIDE[clampi(int(sd.side), 0, 2)]
	var half := fw * 0.5
	var x0 := 0.0 if k == 0 else half
	var top := 110.0 + dy
	var r := Rect2(Vector2(x0, top) * s, Vector2(half, 160.0 - top + 6.0) * s)
	# body
	_grad(r, c[0].darkened(0.1), c[1], 0)
	draw_rect(Rect2(r.position, Vector2(r.size.x, 1.6 * s)), RIM)
	if k == 1:
		draw_rect(Rect2(r.position, Vector2(1.6 * s, r.size.y)), RIM)
	var ext: float = data.get("ext", 0)
	var inner_l := (ext + 2.0 if k == 0 else half + 2.0)
	var inner_r := (half - 2.0 if k == 0 else fw - ext - 2.0)
	# stats chip on the outer side
	var cw := 40.0
	var cx := inner_l if k == 0 else inner_r - cw
	var chip := Rect2(Vector2(cx, top + 3) * s, Vector2(cw, 26) * s)
	_box(chip, Color(0, 0, 0, 0.35), 3, s)
	var rows := [["HIT", int(sd.hit)], ["DMG", int(sd.dmg)], ["CRT", int(sd.crit)]]
	for i in 3:
		var by := (top + 11.0 + 8.0 * i) * s
		_txt(Vector2(chip.position.x + 2.5 * s, by), rows[i][0], int(7.5 * s), LABEL, s)
		_txt(Vector2(chip.end.x - 2.5 * s, by), "--" if rows[i][1] < 0 else str(rows[i][1]), int(9 * s), TEXT, s, 2)
	# weapon pill
	var wx0 := (cx + cw + 3.0) if k == 0 else inner_l
	var wx1 := inner_r if k == 0 else cx - 3.0
	var pill := Rect2(Vector2(wx0, top + 9) * s, Vector2(wx1 - wx0, 17) * s)
	Ui.draw(self, "parch", pill, s)
	var ic: Texture2D = icons[k]
	var tx := pill.position.x + 4 * s
	if ic:
		draw_texture_rect(ic, Rect2(Vector2(tx, pill.position.y + 0.5 * s), Vector2(16, 16) * s), false)
	tx += 17 * s
	var tri: int = sd.tri
	if tri != 0:
		var ac := Color(0.25, 0.8, 0.35) if tri > 0 else Color(0.9, 0.25, 0.25)
		var ay := pill.get_center().y
		var d := -1.0 if tri > 0 else 1.0
		draw_colored_polygon(PackedVector2Array([Vector2(tx, ay - 3 * d * s), Vector2(tx + 6 * s, ay - 3 * d * s), Vector2(tx + 3 * s, ay + 3 * d * s)]), ac)
		tx += 8 * s
	var wn := str(sd.weapon) if str(sd.weapon) != "" else "---"
	# shrink long names to fit the pill instead of cutting them off
	var room := pill.end.x - tx - 3 * s
	var fsz := int(11 * s)
	while fsz > int(6 * s) and font.get_string_size(wn, HORIZONTAL_ALIGNMENT_LEFT, -1, fsz).x > room:
		fsz -= 1
	draw_string(ink_font if ink_font else font, Vector2(tx, pill.position.y + 12.5 * s), wn, HORIZONTAL_ALIGNMENT_LEFT, -1, fsz, INK)
	# HP
	var hp: int = sd.hp
	var mx: int = maxi(1, int(sd.max_hp))
	var hy := top + 31.0
	var hx0 := inner_l + 2.0
	var hx1 := inner_r - 2.0
	_txt(Vector2((hx0) * s, (hy + 12) * s), "HP", int(9 * s), LABEL, s)
	_txt(Vector2((hx0 + 30) * s, (hy + 13) * s), str(hp), int(13 * s), TEXT, s, 2)
	var bar := Rect2(Vector2(hx0 + 34, hy + 4) * s, Vector2(hx1 - hx0 - 34, 8) * s)
	_box(bar, Color(0, 0, 0, 0.55), 3, s)
	# segmented like the original, one notch per HP (up to 60)
	var n := mini(mx, 60)
	var seg := (bar.size.x - 2 * s) / n
	var filled := int(round(float(hp) / mx * n))
	for i in n:
		var col := Color(0.55, 1.0, 0.45) if i < filled else Color(1, 1, 1, 0.1)
		draw_rect(Rect2(bar.position.x + 1 * s + seg * i, bar.position.y + 1.5 * s, maxf(seg - 0.8 * s, 0.6 * s), bar.size.y - 3 * s), col)

func _draw_exp(e: Dictionary, s: float, fw: float, dy: float) -> void:
	var key := "%d>%d" % [e.old, e.new]
	if key != exp_key:
		exp_key = key
		exp_t = 0.0
	var gain: int = e.gain
	var from: float = e.old
	var to: float = from + gain
	var v := lerpf(from, to, clampf(exp_t / 1.2, 0.0, 1.0))
	var shown := fmod(v, 100.0)
	if v >= 100.0 and exp_t >= 1.2:
		shown = float(e.new)
	var w := 130.0
	var r := Rect2(Vector2(fw * 0.5 - w * 0.5, 132 + dy) * s, Vector2(w, 22) * s)
	Ui.draw(self, "win", r, s)
	_txt(Vector2(r.position.x + 5 * s, r.position.y + 15 * s), "EXP", int(10 * s), LABEL, s)
	var bar := Rect2(r.position + Vector2(28, 7) * s, Vector2(w - 56, 8) * s)
	_box(bar, Color(0, 0, 0, 0.55), 3, s)
	_box(Rect2(bar.position + Vector2(1, 1) * s, Vector2((bar.size.x - 2 * s) * shown / 100.0, bar.size.y - 2 * s)), Color(1.0, 0.86, 0.3), 2, s)
	_txt(Vector2(r.end.x - 5 * s, r.position.y + 16 * s), str(int(shown)), int(12 * s), TEXT, s, 2)
	if v >= 100.0:
		_txt(Vector2(r.get_center().x, r.position.y - 3 * s), "LEVEL UP!", int(13 * s), Color(1.0, 0.92, 0.4), s, 1)
