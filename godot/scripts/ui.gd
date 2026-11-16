## The game's own window frames, built at runtime from the tiles in the player's
## ROM (res://ui/frames.json holds only their addresses) and put
## together at any size: corners stay, edges and filling repeat like the tiles
## of the original. Sizes are in original pixels; `unit` is how big one of
## those is in the caller's drawing coordinates.
extends RefCounted

const WHITE := Color(1, 1, 1)
const NUM := Color(0.75, 0.94, 1.0)        # the light blue of the original's numbers
const GREY := Color(0.62, 0.64, 0.74)      # unusable entries
const GREEN := Color(0.55, 1.0, 0.55)
const YELLOW := Color(1.0, 0.97, 0.45)     # stat labels on the status screen
const INK := Color(0.16, 0.14, 0.2)        # text on light boxes
const BROWN := Color(0.39, 0.29, 0.1)
const BAR := Color(0.45, 0.71, 1.0, 0.85)  # stripe under the selected entry

static var meta := {}
static var src := {}
static var cache := {}
static var core: Object        # Fe7Core with the player's ROM loaded
static var spec := {}
static var lzc := {}

## True once the frames can be built (the ROM is loaded).
static func ready() -> bool:
	return core != null and not core.rom_bytes(0, 4).is_empty()

static func _spec() -> Dictionary:
	if spec.is_empty():
		var f := FileAccess.open("res://ui/frames.json", FileAccess.READ)
		if f:
			spec = JSON.parse_string(f.get_as_text())
			for k in spec:
				meta[k] = spec[k].meta
			if spec.has("tab"):
				var m: Array = spec.tab.meta
				meta["parch"] = [m[2], m[1], m[2], m[3], m[4], m[5]]
			if spec.has("bar"):
				meta["barsel"] = spec.bar.meta
	return spec

static func _tile(ts: Array) -> PackedByteArray:
	if ts[0] == "lz":
		var o := int(ts[1])
		if not lzc.has(o):
			lzc[o] = core.rom_lz77(o)
		return (lzc[o] as PackedByteArray).slice(int(ts[2]) * 32, int(ts[2]) * 32 + 32)
	return core.rom_bytes(int(ts[1]), 32)

static func _pal(segs: Array) -> Array:
	var c := []
	c.resize(16)
	c.fill(Color(0, 0, 0, 0))
	for sg in segs:
		var b: PackedByteArray = core.rom_bytes(int(sg[0]), int(sg[2]) * 2)
		for k in int(sg[2]):
			var v := b.decode_u16(2 * k)
			c[int(sg[1]) + k] = Color8((v & 31) * 255 / 31, ((v >> 5) & 31) * 255 / 31, ((v >> 10) & 31) * 255 / 31)
	return c

## Puts one frame piece together from the game's tiles in the player's ROM
## (res://ui/frames.json only says where they are), then cuts it like the
## original windows: corners, edges and a filling that repeats.
static func _build(name: String) -> Image:
	var sp: Dictionary = _spec().get(name, {})
	if sp.is_empty() or not ready():
		return null
	var ops: Array = sp.ops
	var x0 := 99999; var y0 := 99999; var x1 := -99999; var y1 := -99999
	for o in ops:
		x0 = mini(x0, int(o[0])); y0 = mini(y0, int(o[1]))
		x1 = maxi(x1, int(o[0]) + int(o[2])); y1 = maxi(y1, int(o[1]) + int(o[3]))
	var s := Image.create(x1 - x0, y1 - y0, false, Image.FORMAT_RGBA8)
	for c in sp.cells:
		var pal := _pal(c[3])
		var sx := int(c[0]) - x0
		var sy := int(c[1]) - y0
		var ts: Array = c[2]
		if ts[0] == "solid":
			for yy in 8:
				for xx in 8:
					if sx + xx >= 0 and sx + xx < s.get_width() and sy + yy >= 0 and sy + yy < s.get_height():
						s.set_pixel(sx + xx, sy + yy, pal[int(ts[1])])
			continue
		var d := _tile(ts)
		if d.size() < 32:
			continue
		var hf := int(c[4]) == 1
		var vf := int(c[5]) == 1
		for yy in 8:
			for xx in 8:
				var v := (d[yy * 4 + xx / 2] >> (4 * (xx & 1))) & 15
				if v == 0:
					continue
				var X := sx + (7 - xx if hf else xx)
				var Y := sy + (7 - yy if vf else yy)
				if X >= 0 and X < s.get_width() and Y >= 0 and Y < s.get_height():
					s.set_pixel(X, Y, pal[v])
	var m: Array = sp.meta
	var out := Image.create(int(m[0]) + int(m[4]) + int(m[2]), int(m[1]) + int(m[5]) + int(m[3]), false, Image.FORMAT_RGBA8)
	for o in ops:
		out.blit_rect(s, Rect2i(int(o[0]) - x0, int(o[1]) - y0, int(o[2]), int(o[3])), Vector2i(int(o[4]), int(o[5])))
	if name == "narr":
		# the original box darkens towards the top
		var t := int(m[1])
		var hh := int(m[5])
		for y in range(t, t + hh):
			var k := lerpf(0.55, 1.0, float(y - t) / maxf(1.0, hh - 1))
			for x in out.get_width():
				var px := out.get_pixel(x, y)
				if px.a > 0.0 and px.b > px.r + 0.15:
					out.set_pixel(x, y, Color(px.r * k, px.g * k, px.b * lerpf(0.75, 1.0, k), px.a))
	return out

static func _src(name: String) -> Image:
	_spec()
	if not src.has(name) or src[name] == null:
		var img: Image = null
		if name == "parch":
			var tab := _src("tab")
			if tab:
				var m: Array = meta["tab"]
				var l := int(m[0]); var cw := int(m[4]); var r := int(m[2])
				var right := tab.get_region(Rect2i(l + cw, 0, r, tab.get_height()))
				img = Image.create(r + cw + r, tab.get_height(), false, Image.FORMAT_RGBA8)
				var flip := right.duplicate()
				flip.flip_x()
				img.blit_rect(flip, Rect2i(0, 0, r, tab.get_height()), Vector2i(0, 0))
				img.blit_rect(tab, Rect2i(l, 0, cw, tab.get_height()), Vector2i(r, 0))
				img.blit_rect(right, Rect2i(0, 0, r, tab.get_height()), Vector2i(r + cw, 0))
		elif name == "barsel":
			var bar := _src("bar")
			if bar:
				img = bar.duplicate()
				for y in img.get_height():
					for x in img.get_width():
						var c := img.get_pixel(x, y)
						var r8 := c.r8; var g8 := c.g8; var b8 := c.b8
						if c.a > 0.0 and b8 > r8 + 40 and b8 > g8 + 20:
							img.set_pixel(x, y, Color8(mini(255, r8 + 16), mini(255, g8 + 0x58), mini(255, b8 + 0x30), c.a8))
		else:
			img = _build(name)
		if img == null:
			return null
		src[name] = img
	return src[name]

static func image(name: String, w: int, h: int) -> Image:
	var p := _src(name)
	if p == null or not meta.has(name):
		return null
	var m: Array = meta[name]
	var l := int(m[0]); var t := int(m[1]); var r := int(m[2]); var b := int(m[3])
	var cw := int(m[4]); var ch := int(m[5])
	w = maxi(w, l + r + 1)
	h = maxi(h, t + b + 1)
	var o := Image.create(w, h, false, Image.FORMAT_RGBA8)
	var y := t
	while y < h - b:
		var hh := mini(ch, h - b - y)
		var x := l
		while x < w - r:
			o.blit_rect(p, Rect2i(l, t, mini(cw, w - r - x), hh), Vector2i(x, y))
			x += cw
		o.blit_rect(p, Rect2i(0, t, l, hh), Vector2i(0, y))
		o.blit_rect(p, Rect2i(l + cw, t, r, hh), Vector2i(w - r, y))
		y += ch
	var x2 := l
	while x2 < w - r:
		var ww := mini(cw, w - r - x2)
		o.blit_rect(p, Rect2i(l, 0, ww, t), Vector2i(x2, 0))
		o.blit_rect(p, Rect2i(l, t + ch, ww, b), Vector2i(x2, h - b))
		x2 += cw
	o.blit_rect(p, Rect2i(0, 0, l, t), Vector2i(0, 0))
	o.blit_rect(p, Rect2i(l + cw, 0, r, t), Vector2i(w - r, 0))
	o.blit_rect(p, Rect2i(0, t + ch, l, b), Vector2i(0, h - b))
	o.blit_rect(p, Rect2i(l + cw, t + ch, r, b), Vector2i(w - r, h - b))
	return o

static func tex(name: String, w: int, h: int) -> Texture2D:
	var key := "%s:%d:%d" % [name, w, h]
	if not cache.has(key):
		if cache.size() > 400:
			cache.clear()
		var img := image(name, w, h)
		cache[key] = ImageTexture.create_from_image(img) if img else null
	return cache[key]

## Draws the frame `name` over r.
static func draw(ci: CanvasItem, name: String, r: Rect2, unit: float, mod := Color(1, 1, 1)) -> void:
	var t := tex(name, int(round(r.size.x / unit)), int(round(r.size.y / unit)))
	if t:
		ci.draw_texture_rect(t, r, false, mod)

## The light stripe the original puts under the selected entry.
static func bar(ci: CanvasItem, row: Rect2, unit: float) -> void:
	var y := row.position.y + row.size.y - 6.0 * unit
	ci.draw_rect(Rect2(row.position.x, y, row.size.x, 1.0 * unit), Color(0.42, 0.71, 1.0, 0.85))
	ci.draw_rect(Rect2(row.position.x, y + unit, row.size.x, 2.0 * unit), Color(0.52, 0.78, 1.0, 0.85))
	ci.draw_rect(Rect2(row.position.x, y + 3.0 * unit, row.size.x, 1.0 * unit), Color(0.35, 0.61, 0.97, 0.85))

## The see-through windows on the map (goal, unit, terrain).
static func glass(ci: CanvasItem, r: Rect2, unit: float, fill := Color(0.42, 0.42, 0.86, 0.66), rim := Color(1.0, 1.0, 0.81)) -> void:
	var u := unit
	var dark := Color(0.39, 0.35, 0.22, 0.75)
	# drop line right and below
	ci.draw_rect(Rect2(r.end.x, r.position.y + u, u, r.size.y), dark)
	ci.draw_rect(Rect2(r.position.x + u, r.end.y, r.size.x - u, u), dark)
	ci.draw_rect(r.grow(-2.0 * u), fill)
	var rim2 := Color(rim.r * 0.84, rim.g * 0.81, rim.b * 0.6)
	for k in 2:
		var c := rim if k == 0 else rim2
		var q := r.grow(-k * u)
		ci.draw_rect(Rect2(q.position.x, q.position.y, q.size.x, u), c)
		ci.draw_rect(Rect2(q.position.x, q.end.y - u, q.size.x, u), c)
		ci.draw_rect(Rect2(q.position.x, q.position.y + u, u, q.size.y - 2 * u), c)
		ci.draw_rect(Rect2(q.end.x - u, q.position.y + u, u, q.size.y - 2 * u), c)

## A button look for Godot controls from one of the frames (enlarged k times).
## Made before the ROM is loaded, it stays empty and is filled by refresh().
static var pending: Array = []

static func stylebox(name: String, k: int, w := 0, h := 0) -> StyleBoxTexture:
	var sb := StyleBoxTexture.new()
	if not _fill(sb, name, k, w, h):
		pending.append([weakref(sb), name, k, w, h])
	return sb

static func _fill(sb: StyleBoxTexture, name: String, k: int, w: int, h: int) -> bool:
	var p := _src(name)
	if p == null:
		return false
	var m: Array = meta[name]
	var img: Image = p.duplicate()
	if w > 0:
		img = image(name, w, h)
	img.resize(img.get_width() * k, img.get_height() * k, Image.INTERPOLATE_NEAREST)
	sb.texture = ImageTexture.create_from_image(img)
	sb.texture_margin_left = int(m[0]) * k
	sb.texture_margin_top = int(m[1]) * k
	sb.texture_margin_right = int(m[2]) * k
	sb.texture_margin_bottom = int(m[3]) * k
	sb.axis_stretch_horizontal = StyleBoxTexture.AXIS_STRETCH_MODE_TILE
	if sb.axis_stretch_vertical == StyleBoxTexture.AXIS_STRETCH_MODE_STRETCH:
		sb.axis_stretch_vertical = StyleBoxTexture.AXIS_STRETCH_MODE_STRETCH
	return true

## Called once the ROM is loaded: builds the frames for every style made before.
static func refresh() -> void:
	src.clear()
	cache.clear()
	for e in pending:
		var sb: StyleBoxTexture = (e[0] as WeakRef).get_ref()
		if sb:
			_fill(sb, e[1], e[2], e[3], e[4])
	pending.clear()
