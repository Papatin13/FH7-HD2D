## Level-up screen, drawn natively: the battle dims behind a parchment card with
## the unit's portrait (cut from the game's own picture), class and new level,
## and the eight stats; gained points pop in one after another like in the
## original. Coordinates come in frame pixels.
extends Control
const Ui = preload("res://scripts/ui.gd")

const PARCH := Color(0.95, 0.9, 0.76)
const PARCH_DARK := Color(0.62, 0.5, 0.3)
const INK := Color(0.24, 0.15, 0.08)
const BLUE_TOP := Color(0.32, 0.4, 0.88)
const BLUE_BOTTOM := Color(0.16, 0.22, 0.6)
const LABEL := Color(1.0, 0.88, 0.45)
const TEXT := Color(1, 1, 1)
const SHADOW := Color(0, 0, 0, 0.7)
const UP := Color(0.45, 1.0, 0.5)

var font: Font
var title_font: Font
var ink_font: Font
var data := {}
var portrait: Texture2D
var t := 0.0
var key := ""

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_IGNORE

func show_levelup(d: Dictionary, face: Texture2D) -> void:
	data = d
	visible = not d.is_empty()
	if not visible:
		key = ""
		portrait = null
		return
	var k := "%s%d" % [d.name, d.level]
	if k != key:
		key = k
		t = 0.0
		portrait = null
	if face:
		portrait = face
	queue_redraw()

func _process(delta: float) -> void:
	if visible:
		t += delta
		queue_redraw()

func _box(r: Rect2, c: Color, rad: float) -> void:
	var sb := StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(int(rad))
	sb.anti_aliasing = true
	draw_style_box(sb, r)

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
	var fw := size.x / s
	var a := clampf(t * 4.0, 0.0, 1.0)
	draw_rect(Rect2(Vector2.ZERO, size), Color(0.02, 0.03, 0.08, 0.82 * a))
	# card
	var cw := 214.0
	var ch := 120.0
	var c0 := Vector2(fw * 0.5 - cw * 0.5, 22.0 + (1.0 - a) * 12.0)
	var card := Rect2(c0 * s, Vector2(cw, ch) * s)
	Ui.draw(self, "parch", card, s)
	# portrait
	var pr := Rect2(card.position + Vector2(6, 22) * s, Vector2(80, 72) * s)
	_box(pr.grow(1.5 * s), PARCH_DARK, 3 * s)
	_box(pr, Color(0.55, 0.62, 0.78), 2 * s)
	if portrait:
		draw_texture_rect(portrait, pr, false)
	_txt(Vector2(pr.get_center().x, pr.end.y + 15 * s), str(data.name), int(14 * s), INK, s, 1, null, false)
	# header
	_txt(Vector2(card.position.x + 8 * s, card.position.y + 15 * s), "LEVEL UP!", int(13 * s), Color(0.75, 0.35, 0.05), s, 0, title_font, false)
	var hr := Rect2(card.position + Vector2(92, 5) * s, Vector2(cw - 98, 14) * s)
	_box(hr, BLUE_BOTTOM, 4 * s)
	_txt(Vector2(hr.position.x + 4 * s, hr.position.y + 11 * s), str(data["class"]), int(10.5 * s), TEXT, s)
	_txt(Vector2(hr.end.x - 26 * s, hr.position.y + 11 * s), "Lv", int(9 * s), LABEL, s)
	_txt(Vector2(hr.end.x - 4 * s, hr.position.y + 11.5 * s), str(data.level), int(12 * s), TEXT, s, 2)
	# stats panel
	var sp := Rect2(card.position + Vector2(92, 22) * s, Vector2(cw - 98, ch - 28) * s)
	Ui.draw(self, "win", sp, s)
	var stats: Array = data.stats
	var shown := 0
	for i in stats.size():
		var st: Array = stats[i]
		var col := i / 4
		var row := i % 4
		var x := sp.position.x + (4 + 58 * col) * s
		var y := sp.position.y + (15 + 22 * row) * s
		var gain: int = st[2]
		# gains appear one after another
		var appear := 0.6 + 0.35 * shown
		var on := gain != 0 and t >= appear
		if gain != 0:
			shown += 1
		var v: int = st[1] if on or gain == 0 else int(st[1]) - gain
		_txt(Vector2(x, y), str(st[0]), int(9.5 * s), LABEL, s)
		_txt(Vector2(x + 44 * s, y + 0.5 * s), str(v), int(12 * s), TEXT, s, 2)
		if on:
			var pop := clampf((t - appear) * 6.0, 0.0, 1.0)
			var fs := int((9.0 + 4.0 * (1.0 - pop)) * s)
			_txt(Vector2(x + 46 * s, y - 1 * s), "+%d" % gain, fs, UP, s)
