## Info windows of the free battle map, drawn natively in the style of the original:
## chapter goal (top), terrain under the cursor with DEF/AVO, and the unit under
## the cursor with its mini portrait, name and HP bar (blue = own army,
## red = enemy, green = other). Positions follow the game's windows, so their
## slide-in/out animation is kept. Coordinates come in frame pixels.
extends Control
const Ui = preload("res://scripts/ui.gd")

const RIM_LIGHT := Color(0.92, 0.94, 1.0)
const RIM_DARK := Color(0.12, 0.14, 0.32)
const TEXT := Color(0.99, 0.99, 1.0)
const SHADOW := Color(0.05, 0.06, 0.22, 0.8)
const GOAL_TOP := Color(0.62, 0.68, 0.98)
const GOAL_BOTTOM := Color(0.36, 0.42, 0.82)
const STRIP := Color(0.96, 0.93, 0.78)
const STRIP_INK := Color(0.2, 0.3, 0.7)
const NUM_INK := Color(0.16, 0.12, 0.1)
const SIDES := [
	[Color(0.56, 0.7, 1.0), Color(0.3, 0.44, 0.86)],    # own army
	[Color(0.5, 0.86, 0.56), Color(0.22, 0.58, 0.32)],  # other
	[Color(1.0, 0.56, 0.56), Color(0.76, 0.24, 0.28)],  # enemy
]

var font: Font
var ink_font: Font
var title_font: Font
var data := {}
var bg0: Image

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_IGNORE

func show_info(d: Dictionary, bg: Image) -> void:
	data = d
	bg0 = bg
	visible = not d.is_empty()
	if visible:
		queue_redraw()

var _full_size := {}

## While a window slides in or out the game shows only a strip of it. Keep the
## window at its full size (the largest seen) and let it stick out over the edge.
func _whole(ri: Rect2i, key: String, min_size: Vector2i) -> Rect2i:
	var full: Vector2i = _full_size.get(key, min_size)
	full = Vector2i(maxi(full.x, ri.size.x), maxi(full.y, ri.size.y))
	_full_size[key] = full
	var fw := int(size.x / _scale())
	var r := ri
	if r.size.x < full.x:
		if r.position.x + r.size.x / 2 < fw / 2:
			r.position.x = r.end.x - full.x
		r.size.x = full.x
	if r.size.y < full.y:
		if r.position.y + r.size.y / 2 < 80:
			r.position.y = r.end.y - full.y
		r.size.y = full.y
	return r

func _scale() -> float:
	return size.y / 160.0

func _r(ri: Rect2i, s: float) -> Rect2:
	return Rect2(Vector2(ri.position) * s, Vector2(ri.size) * s)

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
		draw_style_box(sb, Rect2(r.position.x, r.position.y + h * i, r.size.x, h + 0.6))

func _frame(r: Rect2, s: float, top: Color, _bottom: Color) -> Rect2:
	# the original's see-through window with a cream rim
	Ui.glass(self, r, s, Color(top.r, top.g, top.b, 0.68))
	return r.grow(-2.0 * s)

func _text(f: Font, p: Vector2, fs: int, txt: String, col: Color, s: float, shadow := true) -> void:
	draw_string(f, p, txt, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, col)

func _centered(f: Font, cx: float, base: float, fs: int, txt: String, col: Color, s: float, maxw: float) -> void:
	if maxw < 8.0 or fs < 2:
		return
	var w := f.get_string_size(txt, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	if w > maxw and w > 0:
		fs = maxi(2, int(fs * maxw / w))
		w = f.get_string_size(txt, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	_text(f, Vector2(cx - w * 0.5, base), fs, txt, col, s)

func _draw() -> void:
	if font == null or data.is_empty():
		return
	var s := _scale()
	if data.has("goal"):
		_draw_goal(data.goal, s)
	if data.has("terrain"):
		_draw_terrain(data.terrain, s)
	if data.has("unit"):
		_draw_unit(data.unit, s)
	if data.has("fights"):
		for f in data.fights:
			_draw_fight(f, s)

func _draw_goal(g: Dictionary, s: float) -> void:
	var l2 := str(g.get("line2", ""))
	# while the window slides in or out only a part of it is on screen:
	# keep its full height and let it stick out over the edge
	var gr: Rect2i = g.rect
	var full := 24 if l2 == "" else 40
	if gr.size.y < full:
		if gr.position.y <= 0:
			gr.position.y = gr.end.y - full
		gr.size.y = full
	var r := _r(gr, s).grow_individual(-2 * s, -2 * s, -2 * s, -1 * s)
	var body := _frame(r, s, GOAL_TOP, GOAL_BOTTOM)
	var f: Font = title_font if title_font else font
	if l2 == "":
		_centered(f, body.get_center().x, body.get_center().y + 4.5 * s, int(13 * s), str(g.text), TEXT, s, body.size.x - 10 * s)
	else:
		_centered(f, body.get_center().x, body.get_center().y - 2.5 * s, int(12 * s), str(g.text), TEXT, s, body.size.x - 10 * s)
		_centered(f, body.get_center().x, body.get_center().y + 11.5 * s, int(12 * s), l2, Color(1.0, 0.9, 0.55), s, body.size.x - 10 * s)

func _draw_terrain(t: Dictionary, s: float) -> void:
	var r := _r(_whole(t.rect, "terrain", Vector2i(48, 56)), s).grow_individual(-2 * s, -4 * s, -2 * s, -2 * s)
	var body := _frame(r, s, GOAL_TOP, GOAL_BOTTOM)
	# name
	_centered(font, body.get_center().x, body.position.y + 14.5 * s, int(13 * s), str(t.name), TEXT, s, body.size.x - 8 * s)
	# DEF / AVO on a light strip
	var strip := Rect2(body.position + Vector2(3 * s, 19 * s), Vector2(body.size.x - 6 * s, body.end.y - body.position.y - 22 * s))
	var sb := StyleBoxFlat.new()
	sb.bg_color = STRIP
	sb.set_corner_radius_all(int(3 * s))
	draw_style_box(sb, strip)
	var rows := [["DEF", int(t.def)], ["AVO", int(t.avo)]]
	var rh := strip.size.y / 2.0
	for i in 2:
		var base := strip.position.y + rh * i + rh * 0.5 + 4 * s
		draw_string(ink_font if ink_font else font, Vector2(strip.position.x + 3 * s, base), rows[i][0], HORIZONTAL_ALIGNMENT_LEFT, -1, int(8.5 * s), STRIP_INK)
		var v := str(rows[i][1])
		var fs := int(10.5 * s)
		var w := (ink_font if ink_font else font).get_string_size(v, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
		draw_string(ink_font if ink_font else font, Vector2(strip.end.x - 3 * s - w, base + 0.5 * s), v, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, NUM_INK)

var face_tex: ImageTexture

## Mini portrait cut from the game's BG0 picture (taken fresh every frame, so it
## always matches what the game shows).
func _face(u: Dictionary) -> Texture2D:
	if not u.has("face") or bg0 == null:
		return null
	var fr: Rect2i = u.face
	if fr.position.x < 0 or fr.position.y < 0 or fr.end.x > 240 or fr.end.y > 160:
		return null
	var img := bg0.get_region(fr)
	if img.is_invisible():
		return null
	if face_tex == null or face_tex.get_size() != Vector2(img.get_size()):
		face_tex = ImageTexture.create_from_image(img)
	else:
		face_tex.update(img)
	return face_tex

func _draw_unit(u: Dictionary, s: float) -> void:
	var r := _r(_whole(u.rect, "unit", Vector2i(96, 48)), s).grow(-2 * s)
	var side: int = clampi(int(u.side), 0, 2)
	var body := _frame(r, s, SIDES[side][0], SIDES[side][1])
	# portrait in a small inset frame
	var fsz := 32.0 * s
	var fp := Vector2(body.position.x + 4 * s, body.get_center().y - fsz * 0.5)
	var inset := StyleBoxFlat.new()
	inset.bg_color = Color(0, 0, 0, 0.25)
	inset.set_corner_radius_all(int(3 * s))
	draw_style_box(inset, Rect2(fp - Vector2(1, 1) * s, Vector2(fsz, fsz) + Vector2(2, 2) * s))
	var tex := _face(u)
	if tex:
		draw_texture_rect(tex, Rect2(fp, Vector2(fsz, fsz)), false)
	var x0 := fp.x + fsz + 4 * s
	var x1 := body.end.x - 4 * s
	var cx := (x0 + x1) * 0.5
	_centered(font, cx, body.position.y + 14 * s, int(13 * s), str(u.name), TEXT, s, x1 - x0)
	# HP x/y
	var hp := int(u.hp)
	var mx := maxi(1, int(u.max_hp))
	var by := body.position.y + 27 * s
	draw_string(font, Vector2(x0, by), "HP", HORIZONTAL_ALIGNMENT_LEFT, -1, int(10 * s), Color(1.0, 0.88, 0.45))
	var v := "%d/%d" % [hp, mx]
	var fs := int(12.5 * s)
	var w := font.get_string_size(v, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	_text(font, Vector2(x1 - w, by + 0.5 * s), fs, v, TEXT, s)
	# HP bar
	var bar := Rect2(Vector2(x0, by + 4 * s), Vector2(x1 - x0, 4 * s))
	var track := StyleBoxFlat.new()
	track.bg_color = Color(0.1, 0.1, 0.18, 0.85)
	track.set_corner_radius_all(int(2 * s))
	draw_style_box(track, bar)
	var frac := clampf(float(hp) / mx, 0.0, 1.0)
	if frac > 0.0:
		var fill := StyleBoxFlat.new()
		fill.bg_color = Color(1.0, 0.86, 0.3) if frac > 0.35 else Color(1.0, 0.45, 0.3)
		fill.set_corner_radius_all(int(2 * s))
		draw_style_box(fill, Rect2(bar.position + Vector2(0.8, 0.8) * s, Vector2((bar.size.x - 1.6 * s) * frac, bar.size.y - 1.6 * s)))

## HP box of a battle fought on the map (no battle animation).
func _draw_fight(u: Dictionary, s: float) -> void:
	var r := _r(u.rect, s).grow(-1 * s)
	var side: int = clampi(int(u.side), 0, 2)
	var body := _frame(r, s, SIDES[side][0], SIDES[side][1])
	var x0 := body.position.x + 4 * s
	var x1 := body.end.x - 4 * s
	_centered(font, (x0 + x1) * 0.5, body.position.y + 11.5 * s, int(11 * s), str(u.name), TEXT, s, x1 - x0)
	var hp := int(u.hp)
	var mx := maxi(1, int(u.max_hp))
	var by := body.end.y - 4 * s
	var v := str(hp)
	var fs := int(11.5 * s)
	_text(font, Vector2(x0, by), fs, v, TEXT, s)
	var bx := x0 + 16 * s
	var bar := Rect2(Vector2(bx, by - 7 * s), Vector2(x1 - bx, 5 * s))
	var track := StyleBoxFlat.new()
	track.bg_color = Color(0.1, 0.1, 0.18, 0.85)
	track.set_corner_radius_all(int(2 * s))
	draw_style_box(track, bar)
	var frac := clampf(float(hp) / mx, 0.0, 1.0)
	if frac > 0.0:
		var fill := StyleBoxFlat.new()
		fill.bg_color = Color(1.0, 0.86, 0.3) if frac > 0.35 else Color(1.0, 0.45, 0.3)
		fill.set_corner_radius_all(int(2 * s))
		draw_style_box(fill, Rect2(bar.position + Vector2(0.8, 0.8) * s, Vector2((bar.size.x - 1.6 * s) * frac, bar.size.y - 1.6 * s)))
