## Map overview (START / M on the map): the whole battle map drawn from the
## game's own map tiles on a parchment card, with unit markers and the cursor.
extends Control

const PARCH := Color(0.94, 0.89, 0.72)
const PARCH_DARK := Color(0.62, 0.5, 0.3)
const INK := Color(0.28, 0.18, 0.08)
const SIDE := [Color(0.25, 0.5, 1.0), Color(0.3, 0.85, 0.35), Color(0.95, 0.25, 0.2)]

var font: Font
var title_font: Font
var data := {}
var map_tex: Texture2D
var margin := 6
var title := ""
var t := 0.0

func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_IGNORE

func show_map(d: Dictionary, tex: Texture2D, m: int, ttl: String) -> void:
	data = d
	map_tex = tex
	margin = m
	title = ttl
	visible = not d.is_empty() and tex != null

func _process(delta: float) -> void:
	t += delta
	if visible:
		queue_redraw()

func _box(r: Rect2, c: Color, rad: int) -> void:
	var sb := StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(rad)
	sb.anti_aliasing = true
	draw_style_box(sb, r)

func _draw() -> void:
	if data.is_empty() or map_tex == null:
		return
	draw_rect(Rect2(Vector2.ZERO, size), Color(0, 0, 0, 0.45))
	var sz: Vector2i = data.size
	var k := minf(size.x * 0.8 / sz.x, size.y * 0.72 / sz.y)
	var ms := Vector2(sz) * k
	var pos := (size - ms) * 0.5 + Vector2(0, size.y * 0.03)
	var card := Rect2(pos - Vector2(18, 58) * (size.y / 900.0), ms + Vector2(36, 76) * (size.y / 900.0))
	var u := size.y / 900.0
	_box(Rect2(card.position + Vector2(6, 8) * u, card.size), Color(0, 0, 0, 0.4), int(18 * u))
	_box(card, PARCH_DARK, int(18 * u))
	_box(card.grow(-6 * u), PARCH, int(14 * u))
	if font and title != "":
		var fs := int(34 * u)
		var f: Font = title_font if title_font else font
		var w := f.get_string_size(title, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
		draw_string(f, Vector2(card.get_center().x - w * 0.5, card.position.y + 42 * u), title, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, INK)
	# the map (the texture has `margin` tiles of border around it)
	var tw := float(map_tex.get_width())
	var th := float(map_tex.get_height())
	var tile := tw / float(sz.x + 2 * margin)
	var src := Rect2(margin * tile, margin * (th / float(sz.y + 2 * margin)), sz.x * tile, sz.y * (th / float(sz.y + 2 * margin)))
	draw_rect(Rect2(pos - Vector2(2, 2) * u, ms + Vector2(4, 4) * u), PARCH_DARK)
	draw_texture_rect_region(map_tex, Rect2(pos, ms), src)
	# units
	for v in data.units:
		var p: Vector3i = v
		var c: Vector2 = pos + (Vector2(p.x, p.y) + Vector2(0.5, 0.5)) * k
		draw_circle(c + Vector2(1, 1.5) * u, k * 0.34, Color(0, 0, 0, 0.5))
		draw_circle(c, k * 0.34, Color(1, 1, 1))
		draw_circle(c, k * 0.27, SIDE[clampi(p.z, 0, 2)])
	# cursor
	var cur: Vector2i = data.cursor
	if cur.x >= 0 and cur.y >= 0 and cur.x < sz.x and cur.y < sz.y:
		var a := 0.6 + 0.4 * sin(t * 5.0)
		draw_rect(Rect2(pos + Vector2(cur) * k, Vector2(k, k)), Color(1, 0.86, 0.35, a), false, maxf(2.0, 3 * u))
