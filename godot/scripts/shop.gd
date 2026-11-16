## Shops (armory, vendor, secret shop) drawn natively over the whole picture,
## with every element at the place of the original one so the mouse handling
## of the game screens keeps working. The shopkeeper portrait is the game's own
## sprite. Coordinates come in frame pixels (240 wide plus the side extension).
extends Control
const Ui = preload("res://scripts/ui.gd")

const TOP := Color(0.33, 0.42, 0.9)
const BOTTOM := Color(0.17, 0.23, 0.62)
const RIM_LIGHT := Color(0.92, 0.94, 1.0)
const RIM_DARK := Color(0.1, 0.12, 0.3)
const TEXT := Color(1, 1, 1)
const VALUE := Color(0.75, 0.94, 1.0)
const GOLDC := Color(1.0, 0.86, 0.35)
const SHADOW := Color(0.04, 0.05, 0.18, 0.8)
const PARCH := Color(0.95, 0.9, 0.76)
const PARCH_DARK := Color(0.62, 0.5, 0.3)
const INK := Color(0.26, 0.16, 0.08)

var font: Font
var title_font: Font
var ink_font: Font
var data := {}
var face: Texture2D
var bg: Texture2D     # the game's background picture
var icon_cb: Callable
var icons := {}
var t := 0.0
var ext := 0.0
var yes_label := "Yes"
var no_label := "No"

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_IGNORE

func show_shop(d: Dictionary, portrait: Texture2D, ext_left: int) -> void:
	data = d
	visible = not d.is_empty()
	ext = ext_left
	if portrait:
		face = portrait
	if not visible:
		face = null
		return
	queue_redraw()

func _process(delta: float) -> void:
	t += delta
	if visible:
		queue_redraw()

func _icon(n: int) -> Texture2D:
	if not icons.has(n) and icon_cb.is_valid():
		icons[n] = icon_cb.call(n)
	return icons.get(n)

func _box(r: Rect2, c: Color, rad: float) -> void:
	var sb := StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(int(rad))
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

func _window(r: Rect2, s: float) -> void:
	Ui.draw(self, "win", r, s)

func _txt(p: Vector2, s: String, fs: int, col: Color, k: float, align := 0, f: Font = null, shadow := true) -> void:
	var ff: Font = f if f else font
	if ink_font and col.v < 0.5:
		ff = ink_font
	var w := ff.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	var x := p.x - (w if align == 2 else (w * 0.5 if align == 1 else 0.0))
	draw_string(ff, Vector2(x, p.y), s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, col)

func _g(x: float, y: float, w: float, h: float, s: float) -> Rect2:
	return Rect2(Vector2(x + ext, y) * s, Vector2(w, h) * s)

func _draw() -> void:
	if font == null or data.is_empty():
		return
	var s := size.y / 160.0
	var arena: bool = data.get("arena", false)
	if not arena:
		# backdrop: warm shop interior (the arena keeps the game's own picture)
		var full := Rect2(Vector2.ZERO, size)
		if bg:
			draw_texture_rect(bg, full, false)
		else:
			_grad(full, Color(0.1, 0.2, 0.36), Color(0.16, 0.36, 0.5), 0)
			for i in 9:
				draw_rect(Rect2(0, size.y * (0.12 + 0.1 * i), size.x, 1.2 * s), Color(0, 0, 0, 0.12))
	# portrait card with the shop's name
	var card := _g(2, 2, 60, 66, s)
	Ui.draw(self, "parch", card, s)
	var fr := _g(8, 6, 48, 48, s)
	_box(fr.grow(1.2 * s), PARCH_DARK, 2 * s)
	if face:
		draw_texture_rect(face, fr, false)
	_txt(Vector2(card.get_center().x, card.end.y - 3.5 * s), str(data.kind), int(9.5 * s), INK, s, 1, null, false)
	# what the shopkeeper says
	var tb := _g(64, 4, 172, 36, s)
	var lines: PackedStringArray = data.text
	for i in mini(lines.size(), 2):
		var plain := lines[i]
		var re := RegEx.create_from_string("\\[[^\\]]*\\]")
		plain = re.sub(plain, "", true)
		_txt(Vector2(tb.position.x + 6 * s, tb.position.y + (14 + 14 * i) * s), plain, int(11.5 * s), TEXT, s)
	var choice: int = data.choice
	if choice >= 0:
		var opts := [yes_label, no_label] if arena else ["Buy", "Sell"]
		for k in 2:
			var ox := (120.0 + 48.0 * k) if arena else (72.0 + 48.0 * k)
			var r := _g(ox, 25, 40, 13, s)
			if k == choice:
				Ui.bar(self, r, s)
				_txt(Vector2(r.get_center().x, r.position.y + 10.5 * s), opts[k], int(11 * s), TEXT, s, 1)
			else:
				_txt(Vector2(r.get_center().x, r.position.y + 10.5 * s), opts[k], int(11 * s), VALUE, s, 1)
	# gold
	var gb := _g(168, 42, 66, 18, s)
	_window(gb, s)
	_txt(Vector2(gb.end.x - 14 * s, gb.position.y + 13 * s), str(int(data.gold)), int(12 * s), TEXT, s, 2)
	_txt(Vector2(gb.end.x - 4 * s, gb.position.y + 13 * s), "G", int(11 * s), GOLDC, s, 2)
	# item list
	if arena:
		return
	var lw := _g(48, 64, 160, 96, s)
	_window(lw, s)
	var items: Array = data.items
	var row: int = data.row
	for i in mini(items.size(), 5):
		var it: Dictionary = items[i]
		var ry := lw.position.y + (6 + 16 * i) * s
		var rr := Rect2(lw.position.x + 4 * s, ry, lw.size.x - 8 * s, 15 * s)
		if i == row:
			Ui.bar(self, Rect2(rr.position.x + 16 * s, rr.position.y, rr.size.x - 16 * s, rr.size.y), s)
		var ic := _icon(int(it.icon))
		if ic:
			draw_texture_rect(ic, Rect2(Vector2(rr.position.x + 1 * s, ry - 0.5 * s), Vector2(16, 16) * s), false)
		_txt(Vector2(rr.position.x + 19 * s, ry + 11.5 * s), str(it.name), int(11.5 * s), TEXT, s)
		var u: int = it.uses
		_txt(Vector2(rr.position.x + 104 * s, ry + 12 * s), "--" if u < 0 else str(u), int(12 * s), VALUE, s, 2)
		var affordable := int(data.mode) == 1 or int(it.price) <= int(data.gold)
		_txt(Vector2(rr.end.x - 2 * s, ry + 12 * s), str(int(it.price)), int(12 * s), VALUE if affordable else Color(0.7, 0.72, 0.8), s, 2)
