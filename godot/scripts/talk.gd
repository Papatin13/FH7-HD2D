## Speech bubbles drawn natively in the style of the original:
## cream bubble with dark rim, rounded corners, tail towards the speaker,
## dark narrow lettering (red highlights), blue "continue" arrow and the
## name plate of the opening scene.
## Sits on top of the game picture; coordinates come in frame pixels.
extends Control
const Ui = preload("res://scripts/ui.gd")

const PAPER := Color(0.937, 0.937, 0.89)
const PAPER_SHADE := Color(0.86, 0.83, 0.74)
const RIM := Color(0.224, 0.192, 0.353)
const INK := Color(0.16, 0.14, 0.13)
const PLATE := Color(0.95, 0.92, 0.72)

var font: Font
var data := {}
var lines_lbl: Array[RichTextLabel] = []
var name_lbl: Label
var t := 0.0
var hand: Texture2D
var choice_lbl: Array[Label] = []

func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	for i in 6:
		var l := RichTextLabel.new()
		l.bbcode_enabled = true
		l.fit_content = true
		l.scroll_active = false
		l.autowrap_mode = TextServer.AUTOWRAP_OFF
		l.mouse_filter = Control.MOUSE_FILTER_IGNORE
		l.add_theme_color_override("default_color", INK)
		add_child(l)
		lines_lbl.append(l)
	for w in ["Yes", "No"]:
		var c := Label.new()
		c.text = w
		c.mouse_filter = Control.MOUSE_FILTER_IGNORE
		c.add_theme_color_override("font_color", INK)
		c.visible = false
		add_child(c)
		choice_lbl.append(c)
	name_lbl = Label.new()
	name_lbl.mouse_filter = Control.MOUSE_FILTER_IGNORE
	name_lbl.add_theme_color_override("font_color", INK)
	name_lbl.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	name_lbl.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	add_child(name_lbl)

func set_font(f: Font) -> void:
	font = f
	for l in lines_lbl:
		l.add_theme_font_override("normal_font", f)
	for c in choice_lbl:
		c.add_theme_font_override("font", f)
	name_lbl.add_theme_font_override("font", f)

## d: Dictionary from Fe7Core.talk(); empty = no bubble.
func show_talk(d: Dictionary) -> void:
	data = d
	visible = not d.is_empty()
	if not visible:
		return
	var s := _scale()
	var r: Rect2 = _r(d.rect)
	var lines: PackedStringArray = d.lines
	var fs := int(round(13.5 * s))
	var tut: bool = d.get("style", 0) == 1
	var pad := (12.0 if tut else 9.0) * s
	# size the text for the whole page (not only the letters printed so far),
	# so it does not jump while it is being written
	var sizing: PackedStringArray = d.get("fit", lines)
	if sizing.size() < lines.size():
		sizing = lines
	if font:
		var maxw := r.size.x - 2.0 * pad - 2.0 * s
		var re := RegEx.create_from_string("\\[[^\\]]*\\]")
		for ln in sizing + lines:
			var plain := re.sub(ln, "", true)
			var wpx := font.get_string_size(plain, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
			if wpx > maxw:
				fs = mini(fs, int(fs * maxw / wpx))
	# line spacing: fit all lines into the box height
	var top := (9.0 if tut else 6.0) * s
	var step := 16.0 * s
	var n := maxi(maxi(sizing.size(), lines.size()), 1)
	var avail := r.size.y - top - (8.0 if tut else 4.0) * s
	if n * step > avail and n > 0:
		step = avail / n
		fs = mini(fs, int(step * 0.95))
	for i in 6:
		var l := lines_lbl[i]
		l.add_theme_font_size_override("normal_font_size", fs)
		l.text = lines[i] if i < lines.size() else ""
		l.position = r.position + Vector2(pad, top + step * i)
		l.size = Vector2(r.size.x - 2.0 * pad, step)
	# yes/no question: the answers on the last line
	var ch: int = d.get("choice", -1)
	for k in 2:
		var c := choice_lbl[k]
		c.visible = ch >= 0
		if ch >= 0:
			var row := mini(lines.size(), 2) - 1
			c.add_theme_font_size_override("font_size", fs)
			c.position = r.position + Vector2((26.0 + 52.0 * k) * s, (4.0 + 16.0 * row) * s)
			c.add_theme_color_override("font_color", INK if k == ch else Color(INK, 0.55))
	name_lbl.visible = str(d.name) != ""
	if name_lbl.visible:
		name_lbl.text = str(d.name)
		name_lbl.add_theme_font_size_override("font_size", int(round(12.0 * s)))
		name_lbl.position = r.position + Vector2(-4.0 * s, -22.0 * s)
		name_lbl.size = Vector2(56.0 * s, 16.0 * s)
	queue_redraw()

func _process(delta: float) -> void:
	t += delta
	if visible and (data.get("waiting", false) or data.get("choice", -1) >= 0):
		queue_redraw()

func _scale() -> float:
	return size.y / 160.0

func _r(ri: Rect2i) -> Rect2:
	var s := _scale()
	return Rect2(Vector2(ri.position) * s, Vector2(ri.size) * s)

func _draw() -> void:
	if data.is_empty():
		return
	var s := _scale()
	if data.get("style", 0) == 1:
		_draw_tutorial(s)
	else:
		_draw_bubble(s)
	_draw_marks(s)

## Yellow tutorial box: pale yellow card with a golden dotted rim.
func _draw_tutorial(s: float) -> void:
	Ui.draw(self, "help", _r(data.rect).grow(-2.0 * s), s)

func _draw_bubble(s: float) -> void:
	var r := _r(data.rect).grow(-1.5 * s)
	var radius := 7.0 * s
	var rim := maxf(1.5, 1.3 * s)
	# drop shadow
	# tail first (the bubble covers its root)
	var tr: Rect2i = data.tail
	if tr.size.x > 0:
		var tail := _r(tr)
		var c := r.get_center()
		var tip: Vector2
		var a: Vector2
		var b: Vector2
		var w := 6.0 * s
		var tc := tail.get_center()
		if tc.x <= r.position.x + 3.0 * s:
			tip = Vector2(tail.position.x + 1.0 * s, tail.get_center().y + 3.0 * s)
			a = Vector2(r.position.x + 1.0 * s, tail.get_center().y - w)
			b = Vector2(r.position.x + 1.0 * s, tail.get_center().y + w)
		elif tc.x >= r.end.x - 3.0 * s:
			tip = Vector2(tail.end.x - 1.0 * s, tail.get_center().y + 3.0 * s)
			a = Vector2(r.end.x - 1.0 * s, tail.get_center().y - w)
			b = Vector2(r.end.x - 1.0 * s, tail.get_center().y + w)
		elif tc.y >= r.end.y - 3.0 * s:
			tip = Vector2(tail.get_center().x, tail.end.y - 1.0 * s)
			a = Vector2(tail.get_center().x - w, r.end.y - 1.0 * s)
			b = Vector2(tail.get_center().x + w, r.end.y - 1.0 * s)
		else:
			tip = Vector2(tail.get_center().x, tail.position.y + 1.0 * s)
			a = Vector2(tail.get_center().x - w, r.position.y + 1.0 * s)
			b = Vector2(tail.get_center().x + w, r.position.y + 1.0 * s)
		draw_colored_polygon(PackedVector2Array([a, tip, b]), PAPER)
		draw_polyline(PackedVector2Array([a, tip, b]), RIM, rim, true)
		# keep the root open
		var inner := (a + b) * 0.5 + (c - (a + b) * 0.5).normalized() * 2.0 * s
		draw_colored_polygon(PackedVector2Array([a.lerp(inner, 0.6), tip.lerp(inner, 0.85), b.lerp(inner, 0.6)]), PAPER)
	# bubble: rim, paper, soft bottom shading
	Ui.draw(self, "bubble", r.grow_individual(0, 0, 0, 1.0 * s), s)

func _draw_marks(s: float) -> void:
	# name plate (opening scene)
	if str(data.name) != "":
		var nr := Rect2(name_lbl.position, name_lbl.size)
		_round_rect(nr.grow(1.0 * s), 3.0 * s, RIM)
		_round_rect(nr, 2.5 * s, PLATE)
	# pointing hand at the chosen answer
	var chs: int = data.get("choice", -1)
	if chs >= 0 and hand:
		var c := choice_lbl[chs]
		var hp := c.position + Vector2(-17.0 * s + sin(t * 6.0) * 1.2 * s, 1.0 * s)
		draw_texture_rect(hand, Rect2(hp, Vector2(16, 16) * s), false)
	# blue "continue" arrow, bouncing
	if data.get("waiting", false):
		var lines: PackedStringArray = data.lines
		var last := lines_lbl[maxi(0, mini(lines.size(), 6) - 1)]
		var tx := last.position.x + last.get_content_width() + 5.0 * s
		var ty := last.position.y + 7.0 * s + sin(t * 7.0) * 1.2 * s
		var tri := PackedVector2Array([Vector2(tx - 3.5 * s, ty), Vector2(tx + 3.5 * s, ty), Vector2(tx, ty + 4.5 * s)])
		draw_colored_polygon(tri, Color(0.25, 0.55, 0.95))
		draw_polyline(PackedVector2Array([tri[0], tri[1], tri[2], tri[0]]), Color(0.1, 0.2, 0.5), maxf(1.0, 0.6 * s), true)

func _round_rect(r: Rect2, radius: float, col: Color) -> void:
	var sb := StyleBoxFlat.new()
	sb.bg_color = col
	sb.set_corner_radius_all(int(maxf(0.0, radius)))
	sb.anti_aliasing = true
	draw_style_box(sb, r)
