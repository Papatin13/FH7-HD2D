## The war room around the map: the battlefield lies as a diorama on a big map
## table, with stone walls, banners, torches and a few things a tactician leaves
## lying around. Everything is built from simple shapes with procedural surfaces,
## so no Game Boy graphics are involved. 1 unit = one map cell.
extends Node3D

const TABLE_Y := -0.8      # table top
const FLOOR_Y := -9.5
const WALL_H := 56.0

var shader: Shader
var lights: Array = []
var flames: Array = []
var t := 0.0

func _mat(kind: int, tint: Color, scale := 1.0) -> ShaderMaterial:
	if shader == null:
		shader = load("res://shaders/room.gdshader")
	var m := ShaderMaterial.new()
	m.shader = shader
	m.set_shader_parameter("kind", kind)
	m.set_shader_parameter("tint", tint)
	m.set_shader_parameter("scale", scale)
	return m

func _box(pos: Vector3, size: Vector3, mat: Material, shadow := false) -> MeshInstance3D:
	var mi := MeshInstance3D.new()
	var b := BoxMesh.new()
	b.size = size
	mi.mesh = b
	mi.position = pos
	mi.material_override = mat
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadow else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(mi)
	return mi

func _cyl(pos: Vector3, r: float, h: float, mat: Material, rot := Vector3.ZERO, shadow := true) -> MeshInstance3D:
	var mi := MeshInstance3D.new()
	var c := CylinderMesh.new()
	c.top_radius = r
	c.bottom_radius = r
	c.height = h
	c.radial_segments = 14
	mi.mesh = c
	mi.position = pos
	mi.rotation_degrees = rot
	mi.material_override = mat
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadow else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(mi)
	return mi

func _flat(col: Color, emit := 0.0) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = col
	m.roughness = 0.8
	if emit > 0.0:
		m.emission_enabled = true
		m.emission = col
		m.emission_energy_multiplier = emit
	return m

func _flame(pos: Vector3, size: float, energy: float, rng: float) -> void:
	var mi := MeshInstance3D.new()
	var s := SphereMesh.new()
	s.radius = size
	s.height = size * 3.2
	s.radial_segments = 8
	s.rings = 4
	mi.mesh = s
	mi.position = pos
	mi.material_override = _flat(Color(1.0, 0.72, 0.3), 3.0)
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(mi)
	var l := OmniLight3D.new()
	l.light_color = Color(1.0, 0.68, 0.36)
	l.light_energy = energy
	l.omni_range = rng
	l.omni_attenuation = 1.3
	l.shadow_enabled = false
	l.position = pos + Vector3(0, size, 0)
	add_child(l)
	lights.append([l, energy, randf() * 10.0])
	flames.append([mi, pos, randf() * 10.0])

func set_quality(q: int) -> void:
	for e in lights:
		(e[0] as OmniLight3D).visible = q > 0

func _process(delta: float) -> void:
	if not is_visible_in_tree():
		return
	t += delta
	for e in lights:
		(e[0] as OmniLight3D).light_energy = e[1] * (0.9 + 0.1 * sin(t * 9.0 + e[2]) * sin(t * 5.3 + e[2] * 2.0))
	for f in flames:
		(f[0] as MeshInstance3D).scale = Vector3(1, 0.92 + 0.12 * sin(t * 11.0 + f[2]), 1)

## x0..x1 / z0..z1: the outer edge of the diorama (map plus its margin).
func build(x0: float, z0: float, x1: float, z1: float) -> void:
	for c in get_children():
		c.queue_free()
	lights.clear()
	flames.clear()
	var cx := (x0 + x1) * 0.5
	var cz := (z0 + z1) * 0.5
	var w := x1 - x0
	var d := z1 - z0
	var wood := _mat(0, Color(0.42, 0.26, 0.13))
	var dark_wood := _mat(0, Color(0.26, 0.15, 0.08))
	var brass := _flat(Color(0.85, 0.66, 0.28))
	brass.metallic = 0.8
	brass.roughness = 0.35

	# frame around the diorama (hides its earth sides)
	var fw := 0.9
	var fh := 0.2 - TABLE_Y
	var fy := TABLE_Y + fh * 0.5
	_box(Vector3(cx, fy, z0 - fw * 0.5), Vector3(w + 2 * fw, fh, fw), dark_wood, true)
	_box(Vector3(cx, fy, z1 + fw * 0.5), Vector3(w + 2 * fw, fh, fw), dark_wood, true)
	_box(Vector3(x0 - fw * 0.5, fy, cz), Vector3(fw, fh, d), dark_wood, true)
	_box(Vector3(x1 + fw * 0.5, fy, cz), Vector3(fw, fh, d), dark_wood, true)
	for sx in [x0 - fw * 0.5, x1 + fw * 0.5]:
		for sz in [z0 - fw * 0.5, z1 + fw * 0.5]:
			_box(Vector3(sx, fy + 0.03, sz), Vector3(fw + 0.12, fh + 0.06, fw + 0.12), brass, true)

	# table
	var border := 6.5
	var tw := w + 2 * (fw + border)
	var td := d + 2 * (fw + border)
	var thick := 1.1
	_box(Vector3(cx, TABLE_Y - thick * 0.5, cz), Vector3(tw, thick, td), wood)
	_box(Vector3(cx, TABLE_Y - thick - 0.5, cz), Vector3(tw - 3.0, 1.0, td - 3.0), dark_wood)
	var leg_h := TABLE_Y - thick - FLOOR_Y
	for sx in [-1.0, 1.0]:
		for sz in [-1.0, 1.0]:
			_box(Vector3(cx + sx * (tw * 0.5 - 2.2), FLOOR_Y + leg_h * 0.5, cz + sz * (td * 0.5 - 2.2)), Vector3(2.2, leg_h, 2.2), dark_wood)

	# things on the table
	var tx0 := cx - tw * 0.5
	var tx1 := cx + tw * 0.5
	var tz0 := cz - td * 0.5
	var tz1 := cz + td * 0.5
	var parch := _mat(3, Color(0.86, 0.78, 0.58), 1.0)
	var wax := _flat(Color(0.93, 0.9, 0.8))
	# candles at two corners
	for p in [Vector3(tx0 + 2.4, 0, tz0 + 2.4), Vector3(tx1 - 2.4, 0, tz0 + 2.6), Vector3(tx0 + 2.6, 0, tz1 - 2.4)]:
		_cyl(Vector3(p.x, TABLE_Y + 0.12, p.z), 0.75, 0.24, brass)
		_cyl(Vector3(p.x, TABLE_Y + 1.1, p.z), 0.3, 1.8, wax)
		_flame(Vector3(p.x, TABLE_Y + 2.35, p.z), 0.16, 1.6, 16.0)
	# rolled maps
	_cyl(Vector3(tx1 - 3.2, TABLE_Y + 0.42, tz1 - 3.0), 0.42, 6.5, parch, Vector3(0, 28, 90))
	_cyl(Vector3(tx1 - 2.6, TABLE_Y + 0.36, tz1 - 4.6), 0.36, 5.0, parch, Vector3(0, -12, 90))
	# an open sheet with a book on it
	_box(Vector3(cx - w * 0.25, TABLE_Y + 0.03, tz1 - 3.2), Vector3(7.0, 0.05, 4.6), parch).rotation_degrees.y = 6
	var book := _box(Vector3(tx0 + 4.2, TABLE_Y + 0.4, cz + 2.0), Vector3(2.6, 0.8, 3.6), _flat(Color(0.45, 0.1, 0.1)), true)
	book.rotation_degrees.y = 14
	_box(Vector3(tx0 + 4.25, TABLE_Y + 0.4, cz + 2.0), Vector3(2.4, 0.62, 3.7), wax).rotation_degrees.y = 14
	# spare figures: little blue and red pawns
	var k := 0
	for col in [Color(0.2, 0.35, 0.85), Color(0.2, 0.35, 0.85), Color(0.8, 0.18, 0.15), Color(0.8, 0.18, 0.15), Color(0.8, 0.18, 0.15)]:
		var px := tx1 - 3.0 - (k % 2) * 1.3
		var pz := cz - 4.0 + k * 1.25
		var m := _flat(col)
		_cyl(Vector3(px, TABLE_Y + 0.1, pz), 0.42, 0.2, m)
		_cyl(Vector3(px, TABLE_Y + 0.6, pz), 0.22, 0.9, m)
		var head := MeshInstance3D.new()
		var sm := SphereMesh.new()
		sm.radius = 0.3
		sm.height = 0.6
		head.mesh = sm
		head.material_override = m
		head.position = Vector3(px, TABLE_Y + 1.2, pz)
		add_child(head)
		k += 1

	# room
	var rw := maxf(tw + 120.0, 150.0)
	var rd := maxf(td + 120.0, 150.0)
	var stone := _mat(1, Color(0.3, 0.29, 0.31))
	var bricks := _mat(2, Color(0.34, 0.31, 0.3))
	_box(Vector3(cx, FLOOR_Y - 0.5, cz), Vector3(rw, 1.0, rd), stone)
	var wy := FLOOR_Y + WALL_H * 0.5
	_box(Vector3(cx, wy, cz - rd * 0.5), Vector3(rw, WALL_H, 1.0), bricks)
	_box(Vector3(cx, wy, cz + rd * 0.5), Vector3(rw, WALL_H, 1.0), bricks)
	_box(Vector3(cx - rw * 0.5, wy, cz), Vector3(1.0, WALL_H, rd), bricks)
	_box(Vector3(cx + rw * 0.5, wy, cz), Vector3(1.0, WALL_H, rd), bricks)
	# carpet under the table
	_box(Vector3(cx, FLOOR_Y + 0.06, cz), Vector3(tw + 18.0, 0.12, td + 18.0), _mat(3, Color(0.72, 0.6, 0.3), 0.3))
	_box(Vector3(cx, FLOOR_Y + 0.1, cz), Vector3(tw + 15.0, 0.14, td + 15.0), _mat(3, Color(0.42, 0.08, 0.1), 0.3))
	# banners, pillars and torches along the walls
	var cols := [Color(0.16, 0.25, 0.62), Color(0.5, 0.1, 0.12), Color(0.14, 0.4, 0.22)]
	var gold := _mat(3, Color(0.8, 0.65, 0.28), 0.3)
	for side in 4:
		var along := rw if side < 2 else rd
		var n := 4
		for i in n:
			var u := (float(i) + 0.5) / n * along - along * 0.5
			var inset := 0.62
			var p: Vector3
			var rot := 0.0
			match side:
				0: p = Vector3(cx + u, 0, cz - rd * 0.5 + inset)
				1: p = Vector3(cx + u, 0, cz + rd * 0.5 - inset); rot = 180.0
				2: p = Vector3(cx - rw * 0.5 + inset, 0, cz + u); rot = 90.0
				_: p = Vector3(cx + rw * 0.5 - inset, 0, cz + u); rot = -90.0
			var ban := _box(Vector3(p.x, FLOOR_Y + 19.0, p.z), Vector3(8.0, 16.0, 0.12), _mat(3, cols[(i + side) % 3], 0.3))
			ban.rotation_degrees.y = rot
			var trim := _box(Vector3(p.x, FLOOR_Y + 27.3, p.z), Vector3(9.0, 0.7, 0.3), gold)
			trim.rotation_degrees.y = rot
			var em := _box(Vector3(p.x, FLOOR_Y + 20.0, p.z) + Vector3(0, 0, 0.1).rotated(Vector3.UP, deg_to_rad(rot)), Vector3(3.2, 3.2, 0.1), gold)
			em.rotation_degrees = Vector3(0, rot, 45)
		# torches between the banners
		for i in n - 1:
			var u := float(i + 1) / n * along - along * 0.5
			var p: Vector3
			var out: Vector3
			match side:
				0: p = Vector3(cx + u, 0, cz - rd * 0.5); out = Vector3(0, 0, 1)
				1: p = Vector3(cx + u, 0, cz + rd * 0.5); out = Vector3(0, 0, -1)
				2: p = Vector3(cx - rw * 0.5, 0, cz + u); out = Vector3(1, 0, 0)
				_: p = Vector3(cx + rw * 0.5, 0, cz + u); out = Vector3(-1, 0, 0)
			var tp := p + out * 1.6 + Vector3(0, FLOOR_Y + 15.0, 0)
			_cyl(tp, 0.35, 3.0, dark_wood, Vector3.ZERO, false)
			_cyl(tp + Vector3(0, 1.6, 0), 0.6, 0.5, brass, Vector3.ZERO, false)
			_flame(tp + Vector3(0, 2.5, 0), 0.5, 5.0, 46.0)
	# pillars in the corners
	for sx in [-1.0, 1.0]:
		for sz in [-1.0, 1.0]:
			_box(Vector3(cx + sx * (rw * 0.5 - 2.0), wy, cz + sz * (rd * 0.5 - 2.0)), Vector3(4.0, WALL_H, 4.0), stone)
