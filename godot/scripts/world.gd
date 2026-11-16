## HD-2D battle map: terrain built from the game's map data and tiles,
## unit sprites as billboards, movement/attack ranges, cursor, light and camera.
extends Node3D

const MARGIN := 6
const PX := 1.0 / 16.0

# Terrain heights (FE7 terrain ids). Natural ground forms smooth slopes,
# buildings/walls/cliffs become blocks.
const NATURAL := {
	0x02: -0.02, 0x0C: 0.04, 0x0D: 0.08, 0x0F: 0.02, 0x10: -0.38, 0x11: 0.8,
	0x12: 1.45, 0x15: -0.45, 0x16: -0.38, 0x1C: 0.05, 0x25: 0.05, 0x26: 0.5,
	0x2F: 0.25, 0x31: -0.2, 0x36: -0.5, 0x3C: -0.38,
}
const BLOCK := {
	0x13: 0.0, 0x14: 0.0, 0x19: 0.25, 0x1A: 1.0, 0x1B: 1.0,
	0x1D: 1.0, 0x1E: 1.0, 0x1F: 0.1, 0x22: 1.0,
	# fence, snag (breakable wall), bridge, barrel, ship rail / brace / mast
	0x32: 0.25, 0x33: 1.0, 0x34: 0.0, 0x39: 0.3, 0x3D: 0.3, 0x3F: 0.5, 0x40: 1.4,
}
# HD-2D standees: the original pixel art stands upright (ground colours cut out),
# the ground below becomes plain grass. Trees per cell, buildings per connected group.
const TREES := [0x0C, 0x0D]
# village, house, armory, vendor, arena, fort, gate, castle gate, church, inn ...
const BUILDINGS := [0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x23, 0x24, 0x2C, 0x2E, 0x30, 0x38]
# tents (Lyn's yurt in the prologue): a soft dome instead of walls and a roof
const TENTS := [0x2C]
const TREE_SCALE := 1.35
const BUILDING_SCALE := 1.3
const WATER_Y := -0.13

var core
var cam: Camera3D
var sun: DirectionalLight3D
var env: WorldEnvironment
var room: Node3D
var attrs: CameraAttributesPractical
var terrain_mi: MeshInstance3D
var water_mi: MeshInstance3D
var range_mi: MeshInstance3D
var cursor_mi: MeshInstance3D
var map_tex: ImageTexture
var cut_tex: ImageTexture
var bld_tex: ImageTexture   # building art with the ground holes filled
var blocks_mi: MeshInstance3D
var bld_mask: Image         # building pixels (ground cut away), for the pop-up buildings
var bld_front := {}         # building cell index -> z of its facade (units stand in front)
var standees: Array = []
var ground_uv := {}   # cell index -> cell index whose tile is used as ground
var atlas_tex: ImageTexture
var sprites: Array = []
var blobs: Array = []
var blob_mat: ShaderMaterial

var gw := 0          # grid size incl. margin
var gh := 0
var map_w := 0
var map_h := 0
var corner_h := PackedFloat32Array()   # (gw+1)*(gh+1)
var block_top := PackedFloat32Array()  # gw*gh, -INF if none
var last_hash := 0
var last_cells := PackedByteArray()
var built_terr := PackedByteArray()   # terrain the 3D shapes were built from
var hash_timer := 0
var cam_target := Vector3.ZERO
var cam_ready := false
var cursor_pos := Vector3.ZERO
var zoom := 1.0
var yaw := 0.0         # degrees, rotation around the map
var pan := Vector3.ZERO  # extra camera offset (tests)
var pitch_deg := 44.0  # camera tilt (90 = straight from above)
# smoothed values actually used
var _zoom_s := 1.0
var _yaw_s := 0.0
var _pitch_s := 44.0

## Upright sprites stand on the ground; seen steeply from above they turn to
## face the camera so they stay readable.
func _stand_mode() -> int:
	return BaseMaterial3D.BILLBOARD_ENABLED if _pitch_s > 60.0 else BaseMaterial3D.BILLBOARD_FIXED_Y

var _last_mode := -1
func _update_standee_mode() -> void:
	var m := _stand_mode()
	if m == _last_mode:
		return
	_last_mode = m
	for sp in standees:
		if sp is Sprite3D:
			(sp as Sprite3D).billboard = m

## Far enough out to see the whole map table, however big the map is.
func max_zoom() -> float:
	return maxf(2.6, float(maxi(gw, gh)) / 9.0)

func reset_view() -> void:
	zoom = 1.0
	yaw = 0.0
	pitch_deg = 44.0
	pan = Vector3.ZERO

## Moves the camera by a screen drag (pixels), relative to the current rotation.
func pan_by(rel: Vector2) -> void:
	var k := 0.03 * _zoom_s
	var r := deg_to_rad(_yaw_s)
	var right := Vector3(cos(r), 0, -sin(r))
	var fwd := Vector3(sin(r), 0, cos(r))
	pan += (-right * rel.x - fwd * rel.y) * k
	pan.x = clampf(pan.x, -map_w * 0.6, map_w * 0.6)
	pan.z = clampf(pan.z, -map_h * 0.6, map_h * 0.6)
var settings := {"dof": true, "shadows": true}

func _ready() -> void:
	cam = Camera3D.new()
	cam.fov = 32.0
	cam.near = 0.1
	cam.far = 400.0
	add_child(cam)
	attrs = CameraAttributesPractical.new()
	cam.attributes = attrs

	sun = DirectionalLight3D.new()
	sun.light_color = Color(1.0, 0.97, 0.9)
	sun.light_energy = 0.8
	sun.shadow_enabled = true
	sun.shadow_blur = 1.5
	sun.directional_shadow_max_distance = 60.0
	sun.rotation_degrees = Vector3(-42, -40, 0)
	add_child(sun)

	env = WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_COLOR
	e.background_color = Color(0.03, 0.025, 0.03)
	e.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	e.ambient_light_color = Color(0.85, 0.9, 1.0)
	e.ambient_light_energy = 0.5
	e.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	e.tonemap_exposure = 1.0
	e.glow_enabled = true
	e.glow_intensity = 0.35
	e.glow_bloom = 0.08
	e.glow_hdr_threshold = 1.1
	e.fog_enabled = true
	e.fog_light_color = Color(0.1, 0.08, 0.07)
	e.fog_density = 0.004
	e.adjustment_enabled = true
	e.adjustment_saturation = 1.08
	e.adjustment_contrast = 1.04
	env.environment = e
	add_child(env)

	terrain_mi = MeshInstance3D.new()
	add_child(terrain_mi)
	room = preload("res://scripts/room.gd").new()
	add_child(room)
	water_mi = MeshInstance3D.new()
	water_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(water_mi)
	range_mi = MeshInstance3D.new()
	range_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(range_mi)

	cursor_mi = MeshInstance3D.new()
	var q := QuadMesh.new()
	q.size = Vector2(1.08, 1.08)
	q.orientation = PlaneMesh.FACE_Y
	cursor_mi.mesh = q
	var cm := ShaderMaterial.new()
	cm.shader = load("res://shaders/cursor.gdshader")
	cm.render_priority = 10
	cursor_mi.material_override = cm
	cursor_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(cursor_mi)

	blob_mat = ShaderMaterial.new()
	blob_mat.shader = load("res://shaders/blob.gdshader")
	map_tex = ImageTexture.new()
	cut_tex = ImageTexture.new()
	bld_tex = ImageTexture.new()
	atlas_tex = ImageTexture.new()
	apply_settings(settings)

func apply_settings(s: Dictionary) -> void:
	settings = s
	var q: int = int(s.get("quality", 2))
	sun.shadow_enabled = s.get("shadows", true) and q > 0
	sun.shadow_blur = 1.5 if q >= 2 else 0.8
	env.environment.glow_enabled = q > 0
	env.environment.fog_enabled = q > 0
	room.set_quality(q)
	attrs.dof_blur_far_enabled = false
	attrs.dof_blur_near_enabled = false

# ---------------------------------------------------------------- terrain

func _cell_index(cx: int, cy: int) -> int:
	return cy * gw + cx

func _natural(t: int) -> float:
	return NATURAL.get(t & 0x7F, 0.0)

func rebuild(core_ref) -> void:
	core = core_ref
	var size: Vector2i = core.map_size()
	if size.x == 0:
		return
	map_w = size.x
	map_h = size.y
	gw = map_w + 2 * MARGIN
	gh = map_h + 2 * MARGIN
	var terr: PackedByteArray = core.terrain(MARGIN)
	built_terr = terr
	# corner heights: average of the natural height of the touching cells
	corner_h.resize((gw + 1) * (gh + 1))
	for vy in gh + 1:
		for vx in gw + 1:
			var s := 0.0
			var n := 0
			for d in [Vector2i(-1, -1), Vector2i(0, -1), Vector2i(-1, 0), Vector2i(0, 0)]:
				var cx: int = vx + d.x
				var cy: int = vy + d.y
				if cx >= 0 and cy >= 0 and cx < gw and cy < gh:
					s += _natural(terr[_cell_index(cx, cy)])
					n += 1
			corner_h[vy * (gw + 1) + vx] = s / max(n, 1)
	block_top.resize(gw * gh)
	for i in gw * gh:
		var t: int = terr[i]
		block_top[i] = BLOCK[t & 0x7F] if BLOCK.has(t & 0x7F) else -INF

	bld_mask = core.map_cutout(MARGIN, PackedByteArray(BUILDINGS))
	bld_front.clear()
	_plan_standees(terr)
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	# raised blocks (walls, pillars, doors ...) use the building texture: the ground
	# showing in their cells is painted over, so no grass ends up on top of a wall
	var bst := SurfaceTool.new()
	bst.begin(Mesh.PRIMITIVE_TRIANGLES)
	var has_blocks := false
	var iw := float(gw * 16)
	var ih := float(gh * 16)
	for cy in gh:
		for cx in gw:
			var outside := cx < MARGIN or cy < MARGIN or cx >= MARGIN + map_w or cy >= MARGIN + map_h
			var shade := 0.82 if outside else 1.0
			var x0 := float(cx - MARGIN)
			var z0 := float(cy - MARGIN)
			var src: int = ground_uv.get(_cell_index(cx, cy), _cell_index(cx, cy))
			var sx := src % gw
			var sy := src / gw
			var uv0 := Vector2(sx * 16 / iw, sy * 16 / ih)
			var uv1 := Vector2((sx + 1) * 16 / iw, (sy + 1) * 16 / ih)
			var h00 := corner_h[cy * (gw + 1) + cx]
			var h10 := corner_h[cy * (gw + 1) + cx + 1]
			var h01 := corner_h[(cy + 1) * (gw + 1) + cx]
			var h11 := corner_h[(cy + 1) * (gw + 1) + cx + 1]
			var bt := block_top[_cell_index(cx, cy)]
			var col := Color(shade, shade, shade)
			if bt > -INF:
				var top := bt + maxf(maxf(h00, h10), maxf(h01, h11))
				var bs: SurfaceTool = bst if bt > 0.05 else st
				has_blocks = has_blocks or bt > 0.05
				_quad(bs, Vector3(x0, top, z0), Vector3(x0 + 1, top, z0), Vector3(x0 + 1, top, z0 + 1), Vector3(x0, top, z0 + 1),
					uv0, Vector2(uv1.x, uv0.y), uv1, Vector2(uv0.x, uv1.y), col)
				# walls down to the surroundings
				var wall := Color(shade * 0.82, shade * 0.82, shade * 0.86)
				var sides := [
					[Vector2i(0, 1), Vector3(x0, 0, z0 + 1), Vector3(x0 + 1, 0, z0 + 1), h01, h11],
					[Vector2i(0, -1), Vector3(x0 + 1, 0, z0), Vector3(x0, 0, z0), h10, h00],
					[Vector2i(1, 0), Vector3(x0 + 1, 0, z0 + 1), Vector3(x0 + 1, 0, z0), h11, h10],
					[Vector2i(-1, 0), Vector3(x0, 0, z0), Vector3(x0, 0, z0 + 1), h00, h01],
				]
				for sd in sides:
					var nx: int = cx + sd[0].x
					var ny: int = cy + sd[0].y
					if nx >= 0 and ny >= 0 and nx < gw and ny < gh:
						var nb := block_top[_cell_index(nx, ny)]
						if nb > -INF and absf(nb - bt) < 0.01:
							continue
					var a: Vector3 = sd[1]
					var b: Vector3 = sd[2]
					var ha: float = sd[3]
					var hb: float = sd[4]
					_quad(bs, Vector3(a.x, top, a.z), Vector3(b.x, top, b.z), Vector3(b.x, hb, b.z), Vector3(a.x, ha, a.z),
						Vector2(uv0.x, (uv0.y + uv1.y) * 0.5), Vector2(uv1.x, (uv0.y + uv1.y) * 0.5), uv1, Vector2(uv0.x, uv1.y), wall)
			else:
				_quad(st, Vector3(x0, h00, z0), Vector3(x0 + 1, h10, z0), Vector3(x0 + 1, h11, z0 + 1), Vector3(x0, h01, z0 + 1),
					uv0, Vector2(uv1.x, uv0.y), uv1, Vector2(uv0.x, uv1.y), col)
	# diorama skirt around everything
	var e0 := Vector3(-MARGIN, 0, -MARGIN)
	var e1 := Vector3(map_w + MARGIN, 0, map_h + MARGIN)
	var dirt := Color(0.18, 0.15, 0.13)
	var duv := Vector2(0.001, 0.001)
	for k in 4:
		var a: Vector3
		var b: Vector3
		match k:
			0: a = Vector3(e0.x, 0, e1.z); b = Vector3(e1.x, 0, e1.z)
			1: a = Vector3(e1.x, 0, e0.z); b = Vector3(e0.x, 0, e0.z)
			2: a = Vector3(e1.x, 0, e1.z); b = Vector3(e1.x, 0, e0.z)
			_: a = Vector3(e0.x, 0, e0.z); b = Vector3(e0.x, 0, e1.z)
		_quad(st, a, b, b + Vector3(0, -3, 0), a + Vector3(0, -3, 0), duv, duv, duv, duv, dirt)
	st.generate_normals()
	var mesh := st.commit()
	var mat := StandardMaterial3D.new()
	mat.albedo_texture = map_tex
	mat.vertex_color_use_as_albedo = true
	mat.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	mat.roughness = 0.95
	mat.specular_mode = BaseMaterial3D.SPECULAR_DISABLED
	terrain_mi.mesh = mesh
	terrain_mi.material_override = mat
	if blocks_mi == null:
		blocks_mi = MeshInstance3D.new()
		add_child(blocks_mi)
	blocks_mi.visible = has_blocks
	if has_blocks:
		bst.generate_normals()
		blocks_mi.mesh = bst.commit()
		var bmat := mat.duplicate() as StandardMaterial3D
		bmat.albedo_texture = bld_tex
		blocks_mi.material_override = bmat

	# water surface
	var wm := PlaneMesh.new()
	wm.size = Vector2(gw, gh)
	water_mi.mesh = wm
	water_mi.position = Vector3(map_w * 0.5, WATER_Y, map_h * 0.5)
	var wmat := StandardMaterial3D.new()
	wmat.albedo_color = Color(0.25, 0.5, 0.8, 0.35)
	wmat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	wmat.roughness = 0.08
	wmat.metallic = 0.3
	wmat.emission_enabled = true
	wmat.emission = Color(0.05, 0.12, 0.2)
	water_mi.material_override = wmat
	room.build(-MARGIN, -MARGIN, map_w + MARGIN, map_h + MARGIN)
	zoom = minf(zoom, max_zoom())
	room.set_quality(int(settings.get("quality", 2)))
	last_hash = 0
	update_map_texture(true)
	last_cells = PackedByteArray()
	cam_ready = false

## Finds tree cells and building groups, picks a grass tile for the ground under them
## and creates the upright sprites.
func _plan_standees(terr: PackedByteArray) -> void:
	for sp in standees:
		sp.queue_free()
	standees.clear()
	ground_uv.clear()
	var inside := func(cx: int, cy: int) -> bool:
		return cx >= MARGIN and cy >= MARGIN and cx < MARGIN + map_w and cy < MARGIN + map_h
	var kind := func(cx: int, cy: int) -> int:
		if cx < 0 or cy < 0 or cx >= gw or cy >= gh:
			return 0
		var t: int = terr[_cell_index(cx, cy)] & 0x7F
		if t in TREES:
			return 1
		if t in BUILDINGS:
			return 2
		return 0
	# ground: nearest plain cell
	for cy in gh:
		for cx in gw:
			if kind.call(cx, cy) == 0:
				continue
			var best := -1
			for r in range(1, 6):
				for dy in range(-r, r + 1):
					for dx in range(-r, r + 1):
						var nx := cx + dx
						var ny := cy + dy
						if nx >= 0 and ny >= 0 and nx < gw and ny < gh and (terr[_cell_index(nx, ny)] & 0x7F) == 0x01:
							best = _cell_index(nx, ny)
							break
					if best >= 0:
						break
				if best >= 0:
					break
			if best >= 0:
				ground_uv[_cell_index(cx, cy)] = best
	# trees: one per cell; buildings: one per connected group (up to 4x4)
	var seen := {}
	for cy in gh:
		for cx in gw:
			var k: int = kind.call(cx, cy)
			if k == 0 or seen.has(_cell_index(cx, cy)):
				continue
			var x0 := cx
			var y0 := cy
			var x1 := cx
			var y1 := cy
			if k == 2:
				var stack := [Vector2i(cx, cy)]
				seen[_cell_index(cx, cy)] = true
				while not stack.is_empty():
					var c: Vector2i = stack.pop_back()
					x0 = mini(x0, c.x)
					y0 = mini(y0, c.y)
					x1 = maxi(x1, c.x)
					y1 = maxi(y1, c.y)
					for d in [Vector2i(1, 0), Vector2i(-1, 0), Vector2i(0, 1), Vector2i(0, -1)]:
						var n: Vector2i = c + d
						if kind.call(n.x, n.y) == 2 and not seen.has(_cell_index(n.x, n.y)):
							seen[_cell_index(n.x, n.y)] = true
							stack.append(n)
			if k == 1:
				_add_standee(x0, y0, x1, y1, true)
			else:
				var tent := false
				for yy in range(y0, y1 + 1):
					for xx in range(x0, x1 + 1):
						tent = tent or (terr[_cell_index(xx, yy)] & 0x7F) in TENTS
				_add_building(x0, y0, x1, y1, false, tent)

## Real 3D building from the original tile art: the upper part of the art is the
## roof, the lower part the front wall (door, windows). Single cells get a gable
## roof, everything else pops up from the art (see _popup).
func _add_building(x0: int, y0: int, x1: int, y1: int, flat := false, tent := false) -> void:
	var iw := float(gw * 16)
	var ih := float(gh * 16)
	var px0 := x0 * 16 + 0.5
	var px1 := (x1 + 1) * 16 - 0.5
	var py0 := y0 * 16 + 0.5
	var py1 := (y1 + 1) * 16 - 0.5
	var u0 := px0 / iw
	var u1 := px1 / iw
	var ins := 0.0 if flat else 0.06
	var wx0 := float(x0 - MARGIN) + ins
	var wx1 := float(x1 + 1 - MARGIN) - ins
	var wz0 := float(y0 - MARGIN) + ins
	var wz1 := float(y1 + 1 - MARGIN) - ins
	var base := height_at((wx0 + wx1) * 0.5, (wz0 + wz1) * 0.5) - 0.02
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var white := Color(1, 1, 1)
	var side := Color(0.86, 0.86, 0.9)
	var pieces := _building_pieces(x0 * 16, y0 * 16, (x1 - x0 + 1) * 16, (y1 - y0 + 1) * 16)
	# a lone house that fills its cell gets a gable roof, everything else pops up
	var single := x1 == x0 and y1 == y0 and pieces.size() == 1 and int(pieces[0][1]) >= 200 and not tent
	if single:
		var split := py0 + (py1 - py0) * 0.55   # roof above, wall below
		var vr0 := py0 / ih
		var vs := split / ih
		var v1 := py1 / ih
		var hw := 0.5
		var hr := 0.42
		var zm := (wz0 + wz1) * 0.5
		var top := base + hw
		var ov := 0.07
		# walls: front/back show the facade, sides the middle of it
		_quad(st, Vector3(wx0, top, wz1), Vector3(wx1, top, wz1), Vector3(wx1, base, wz1), Vector3(wx0, base, wz1),
			Vector2(u0, vs), Vector2(u1, vs), Vector2(u1, v1), Vector2(u0, v1), white)
		_quad(st, Vector3(wx1, top, wz0), Vector3(wx0, top, wz0), Vector3(wx0, base, wz0), Vector3(wx1, base, wz0),
			Vector2(u0, vs), Vector2(u1, vs), Vector2(u1, v1), Vector2(u0, v1), side)
		var um0 := lerpf(u0, u1, 0.25)
		var um1 := lerpf(u0, u1, 0.75)
		_quad(st, Vector3(wx0, top, wz0), Vector3(wx0, top, wz1), Vector3(wx0, base, wz1), Vector3(wx0, base, wz0),
			Vector2(um0, vs), Vector2(um1, vs), Vector2(um1, v1), Vector2(um0, v1), side)
		_quad(st, Vector3(wx1, top, wz1), Vector3(wx1, top, wz0), Vector3(wx1, base, wz0), Vector3(wx1, base, wz1),
			Vector2(um0, vs), Vector2(um1, vs), Vector2(um1, v1), Vector2(um0, v1), side)
		# gable ends
		for gx in [wx0, wx1]:
			var a := Vector3(gx, top, wz0)
			var b := Vector3(gx, top, wz1)
			var c := Vector3(gx, top + hr, zm)
			for v in [[a, Vector2(um0, vs)], [b, Vector2(um1, vs)], [c, Vector2((um0 + um1) * 0.5, lerpf(vs, vr0, 0.4))]]:
				st.set_color(side)
				st.set_uv(v[1])
				st.add_vertex(v[0])
		# roof: two slopes with the roof art, a little overhang
		var r0 := wx0 - ov
		var r1 := wx1 + ov
		_quad(st, Vector3(r0, top + hr, zm), Vector3(r1, top + hr, zm), Vector3(r1, top - 0.05, wz1 + ov), Vector3(r0, top - 0.05, wz1 + ov),
			Vector2(u0, vr0), Vector2(u1, vr0), Vector2(u1, vs), Vector2(u0, vs), white)
		_quad(st, Vector3(r1, top + hr, zm), Vector3(r0, top + hr, zm), Vector3(r0, top - 0.05, wz0 - ov), Vector3(r1, top - 0.05, wz0 - ov),
			Vector2(u0, vr0), Vector2(u1, vr0), Vector2(u1, vs), Vector2(u0, vs), side)
	elif pieces.is_empty():
		return
	else:
		for pc in pieces:
			if tent:
				_dome(st, x0, y0, x1, y1, base, pc[0])
			else:
				_popup(st, x0, y0, x1, y1, base, pc[0])
	st.generate_normals()
	var mi := MeshInstance3D.new()
	mi.mesh = st.commit()
	var mat := StandardMaterial3D.new()
	mat.albedo_texture = bld_tex
	mat.vertex_color_use_as_albedo = true
	mat.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mat.roughness = 0.9
	mi.material_override = mat
	add_child(mi)
	standees.append(mi)

## Pop-up building from the top-down art: per pixel column the lowest rows are the
## front wall (door, windows) standing upright on the ground, the rows above are the
## roof lying flat on top of it. The outline follows the art, so round arenas get a
## round wall and no grass from the cell corners ends up on walls or roofs.
func _popup(st: SurfaceTool, x0: int, y0: int, x1: int, y1: int, base: float, m: PackedByteArray) -> bool:
	var iw := float(gw * 16)
	var ih := float(gh * 16)
	var px0 := x0 * 16
	var px1 := (x1 + 1) * 16
	var py0 := y0 * 16
	var py1 := (y1 + 1) * 16
	var n := px1 - px0
	var t := PackedInt32Array()
	var f := PackedInt32Array()
	t.resize(n)
	f.resize(n)
	var tallest := 0
	for i in n:
		t[i] = -1
		f[i] = -1
		for y in range(py0, py1):
			if m[(y - py0) * n + i]:
				if t[i] < 0:
					t[i] = y
				f[i] = y + 1
		if t[i] >= 0:
			tallest = maxi(tallest, f[i] - t[i])
	if tallest == 0:
		return false
	# one wall height for the whole building: about the lower 40 % of the art
	var wall_px := clampi(int(round(tallest * 0.4)), 4, 11)
	var hw := wall_px * PX * 1.35
	var top := base + hw
	var white := Color(1, 1, 1)
	var side := Color(0.8, 0.8, 0.86)
	var back := Color(0.62, 0.62, 0.7)
	var zof := func(py: float) -> float: return py / 16.0 - MARGIN
	for i in n:
		if t[i] < 0:
			continue
		var c := px0 + i
		var xa := float(c) / 16.0 - MARGIN
		var xb := xa + PX
		var u0 := (c + 0.1) / iw
		var u1 := (c + 0.9) / iw
		var fw := mini(wall_px, f[i] - t[i])        # facade rows of this column
		var vt := t[i] / ih
		var vw := float(f[i] - fw) / ih              # roof ends, wall begins
		var vf := (f[i] - 0.01) / ih
		var zt: float = zof.call(t[i])
		var zf: float = zof.call(f[i])
		# roof (if the column is taller than the wall) and front wall
		if f[i] - fw > t[i]:
			_quad(st, Vector3(xa, top, zt), Vector3(xb, top, zt), Vector3(xb, top, zf), Vector3(xa, top, zf),
				Vector2(u0, vt), Vector2(u1, vt), Vector2(u1, vw), Vector2(u0, vw), white)
		else:
			_quad(st, Vector3(xa, top, zt), Vector3(xb, top, zt), Vector3(xb, top, zf), Vector3(xa, top, zf),
				Vector2(u0, vw), Vector2(u1, vw), Vector2(u1, vw), Vector2(u0, vw), white)
		_quad(st, Vector3(xa, top, zf), Vector3(xb, top, zf), Vector3(xb, base, zf), Vector3(xa, base, zf),
			Vector2(u0, vw), Vector2(u1, vw), Vector2(u1, vf), Vector2(u0, vf), white)
		# back wall, in the colour of the roof's back edge
		_quad(st, Vector3(xb, top, zt), Vector3(xa, top, zt), Vector3(xa, base, zt), Vector3(xb, base, zt),
			Vector2(u0, vt), Vector2(u1, vt), Vector2(u1, vt + PX / ih), Vector2(u0, vt + PX / ih), back)
		# side walls where the neighbouring column does not cover this one
		for dir in [-1, 1]:
			var j: int = i + dir
			var xs := xa if dir < 0 else xb
			var segs := []
			if j < 0 or j >= n or t[j] < 0:
				segs.append([t[i], f[i]])
			else:
				if t[j] > t[i]:
					segs.append([t[i], mini(t[j], f[i])])
				if f[j] < f[i]:
					segs.append([maxi(f[j], t[i]), f[i]])
			for sg in segs:
				if sg[1] <= sg[0]:
					continue
				var za: float = zof.call(sg[0])
				var zb: float = zof.call(sg[1])
				var a := Vector3(xs, top, za)
				var b := Vector3(xs, top, zb)
				if dir > 0:
					var tmp := a
					a = b
					b = tmp
				_quad(st, a, b, Vector3(b.x, base, b.z), Vector3(a.x, base, a.z),
					Vector2(u0, vw), Vector2(u1, vw), Vector2(u1, vf), Vector2(u0, vf), side)
		# units on a building cell stand in front of its wall
		var cell := _cell_index(c / 16, (f[i] - 1) / 16)
		bld_front[cell] = maxf(float(bld_front.get(cell, -INF)), zf)
	return true

## The buildings in a rectangle of the art: ground is already cut away, bushes and
## hedges (green) are dropped too, and each connected piece of at least 40 pixels
## becomes its own building ([mask, pixel count]); loose bits stay on the ground.
func _building_pieces(px: int, py: int, w: int, h: int) -> Array:
	var out := []
	if bld_mask == null:
		return out
	var m := PackedByteArray()
	m.resize(w * h)
	for y in h:
		for x in w:
			var c := bld_mask.get_pixel(px + x, py + y)
			if c.a > 0.5 and not (c.g > c.r + 0.08 and c.g > c.b - 0.12):
				m[y * w + x] = 1
	var comp := PackedInt32Array()
	comp.resize(w * h)
	comp.fill(-1)
	var counts := []
	for k in w * h:
		if m[k] == 0 or comp[k] >= 0:
			continue
		var id := counts.size()
		var stack := [k]
		comp[k] = id
		var cnt := 0
		while not stack.is_empty():
			var q: int = stack.pop_back()
			cnt += 1
			var qx := q % w
			var qy := q / w
			for d in [[1, 0], [-1, 0], [0, 1], [0, -1]]:
				var nx: int = qx + d[0]
				var ny: int = qy + d[1]
				if nx >= 0 and ny >= 0 and nx < w and ny < h:
					var nk := ny * w + nx
					if m[nk] == 1 and comp[nk] < 0:
						comp[nk] = id
						stack.append(nk)
		counts.append(cnt)
	for id in counts.size():
		if counts[id] < 40:
			continue
		var pm := PackedByteArray()
		pm.resize(w * h)
		for k in w * h:
			if comp[k] == id:
				pm[k] = 1
		out.append([pm, counts[id]])
	return out

## Tent: the art laid over a dome that rises from its outline (steep at the
## edge, round on top), seen from above like the original picture.
func _dome(st: SurfaceTool, x0: int, y0: int, x1: int, y1: int, base: float, m: PackedByteArray) -> void:
	var iw := float(gw * 16)
	var ih := float(gh * 16)
	var px0 := x0 * 16
	var py0 := y0 * 16
	var w := (x1 - x0 + 1) * 16
	var h := (y1 - y0 + 1) * 16
	# distance of every pixel to the outside (chamfer, two passes)
	var d := PackedFloat32Array()
	d.resize(w * h)
	for k in w * h:
		d[k] = 1e6 if m[k] else 0.0
	for y in h:
		for x in w:
			var k := y * w + x
			if d[k] == 0.0:
				continue
			var v := 1.0 if x == 0 or y == 0 else minf(d[k - 1] + 1.0, d[k - w] + 1.0)
			if x > 0 and y > 0:
				v = minf(v, d[k - w - 1] + 1.4142)
			if x < w - 1 and y > 0:
				v = minf(v, d[k - w + 1] + 1.4142)
			d[k] = minf(d[k], v)
	for y in range(h - 1, -1, -1):
		for x in range(w - 1, -1, -1):
			var k := y * w + x
			if d[k] == 0.0:
				continue
			var v := 1.0 if x == w - 1 or y == h - 1 else minf(d[k + 1] + 1.0, d[k + w] + 1.0)
			if x < w - 1 and y < h - 1:
				v = minf(v, d[k + w + 1] + 1.4142)
			if x > 0 and y < h - 1:
				v = minf(v, d[k + w - 1] + 1.4142)
			d[k] = minf(d[k], v)
	var dmax := 1.0
	for k in w * h:
		dmax = maxf(dmax, d[k])
	var top := clampf(dmax * 0.09, 0.35, 0.95)
	# height at pixel corners: mean of the four touching pixels
	var hc := PackedFloat32Array()
	hc.resize((w + 1) * (h + 1))
	for y in h + 1:
		for x in w + 1:
			var sum := 0.0
			for o in [[-1, -1], [0, -1], [-1, 0], [0, 0]]:
				var xx: int = x + o[0]
				var yy: int = y + o[1]
				if xx >= 0 and yy >= 0 and xx < w and yy < h:
					sum += d[yy * w + xx]
			sum *= 0.25
			hc[y * (w + 1) + x] = top * sqrt(clampf(sum / dmax, 0.0, 1.0))
	var white := Color(1, 1, 1)
	for y in h:
		for x in w:
			if not m[y * w + x]:
				continue
			var gx := float(px0 + x)
			var gy := float(py0 + y)
			var xa := gx / 16.0 - MARGIN
			var za := gy / 16.0 - MARGIN
			var u0 := gx / iw
			var v0 := gy / ih
			var u1 := (gx + 1.0) / iw
			var v1 := (gy + 1.0) / ih
			_quad(st, Vector3(xa, base + hc[y * (w + 1) + x], za), Vector3(xa + PX, base + hc[y * (w + 1) + x + 1], za),
				Vector3(xa + PX, base + hc[(y + 1) * (w + 1) + x + 1], za + PX), Vector3(xa, base + hc[(y + 1) * (w + 1) + x], za + PX),
				Vector2(u0, v0), Vector2(u1, v0), Vector2(u1, v1), Vector2(u0, v1), white)
			var cell := _cell_index((px0 + x) / 16, (py0 + y) / 16)
			bld_front[cell] = maxf(float(bld_front.get(cell, -INF)), za + PX)

func _add_standee(x0: int, y0: int, x1: int, y1: int, tree: bool) -> void:
	var s := Sprite3D.new()
	s.texture = cut_tex
	s.region_enabled = true
	var w := (x1 - x0 + 1) * 16
	var h := (y1 - y0 + 1) * 16
	s.region_rect = Rect2(x0 * 16, y0 * 16, w, h)
	s.pixel_size = PX
	s.offset = Vector2(0, h * 0.5)
	s.billboard = BaseMaterial3D.BILLBOARD_FIXED_Y
	s.alpha_cut = SpriteBase3D.ALPHA_CUT_DISCARD
	s.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	s.shaded = false
	s.double_sided = true
	s.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	var wx := (x0 + x1 + 1) * 0.5 - MARGIN
	var wz := float(y1 - MARGIN) + 0.42
	if tree:
		# a little irregularity so forests do not look like a grid
		var j := sin(float(x0 * 12.9898 + y0 * 78.233)) * 43758.5
		wx += (j - floor(j) - 0.5) * 0.16
	s.position = Vector3(wx, height_at(clampf(wx, x0 - MARGIN + 0.1, x1 - MARGIN + 0.9), wz) - 0.02, wz)
	var sc := TREE_SCALE if tree else BUILDING_SCALE
	s.scale = Vector3(1.0, sc, 1.0)
	s.billboard = _stand_mode()
	add_child(s)
	standees.append(s)

func _quad(st: SurfaceTool, a: Vector3, b: Vector3, c: Vector3, d: Vector3, ua: Vector2, ub: Vector2, uc: Vector2, ud: Vector2, col: Color) -> void:
	for v in [[a, ua], [b, ub], [c, uc], [a, ua], [c, uc], [d, ud]]:
		st.set_color(col)
		st.set_uv(v[1])
		st.add_vertex(v[0])

func update_map_texture(force := false) -> void:
	var h: int = core.map_hash()
	if not force and h == last_hash:
		return
	last_hash = h
	var img: Image = core.map_image(MARGIN)
	if img == null:
		return
	if map_tex.get_width() == img.get_width() and map_tex.get_height() == img.get_height():
		map_tex.update(img)
	else:
		map_tex.set_image(img)
	var kinds := PackedByteArray(TREES + BUILDINGS)
	var cut: Image = core.map_cutout(MARGIN, kinds)
	if cut != null:
		if cut_tex.get_width() == cut.get_width() and cut_tex.get_height() == cut.get_height():
			cut_tex.update(cut)
		else:
			cut_tex.set_image(cut)
	var blocks := _solid_kinds()
	for t in BUILDINGS:
		blocks.erase(t)
	var bld: Image = core.building_tex(MARGIN, PackedByteArray(_solid_kinds()), PackedByteArray(blocks))
	if bld != null:
		if bld_tex.get_width() == bld.get_width() and bld_tex.get_height() == bld.get_height():
			bld_tex.update(bld)
		else:
			bld_tex.set_image(bld)

## Terrain whose cells get their ground painted over: buildings and raised blocks.
func _solid_kinds() -> Array:
	var k: Array = BUILDINGS.duplicate()
	for t in BLOCK:
		if BLOCK[t] > 0.05:
			k.append(t)
	return k

## Ground height at a map position (world units = cells).
func height_at(x: float, z: float) -> float:
	if gw == 0:
		return 0.0
	var gx := clampf(x + MARGIN, 0.0, gw - 0.001)
	var gz := clampf(z + MARGIN, 0.0, gh - 0.001)
	var cx := int(gx)
	var cy := int(gz)
	var fx := gx - cx
	var fz := gz - cy
	var h00 := corner_h[cy * (gw + 1) + cx]
	var h10 := corner_h[cy * (gw + 1) + cx + 1]
	var h01 := corner_h[(cy + 1) * (gw + 1) + cx]
	var h11 := corner_h[(cy + 1) * (gw + 1) + cx + 1]
	var bt := block_top[_cell_index(cx, cy)]
	if bt > -INF:
		return bt + maxf(maxf(h00, h10), maxf(h01, h11))
	return lerpf(lerpf(h00, h10, fx), lerpf(h01, h11, fx), fz)

# ---------------------------------------------------------------- per frame

func update_frame(delta: float) -> void:
	hash_timer += 1
	if hash_timer % 4 == 0:
		update_map_texture()
	if hash_timer % 12 == 0:
		# the terrain itself changed (new chapter of the same size, a wall broken,
		# a door opened, a village destroyed): build the 3D shapes again
		var terr: PackedByteArray = core.terrain(MARGIN)
		if terr != built_terr:
			var keep := cam_ready
			rebuild(core)
			cam_ready = keep
	_update_sprites()
	_update_range()
	_update_cursor(delta)
	_update_camera(delta)

func _update_sprites() -> void:
	var data: PackedInt32Array = core.sprites()
	var n := data.size() / 7
	if n > 0:
		var img: Image = core.atlas()
		if img != null:
			if atlas_tex.get_width() == img.get_width():
				atlas_tex.update(img)
			else:
				atlas_tex.set_image(img)
	while sprites.size() < n:
		var s := Sprite3D.new()
		s.texture = atlas_tex
		s.region_enabled = true
		s.pixel_size = PX
		s.billboard = BaseMaterial3D.BILLBOARD_FIXED_Y
		s.alpha_cut = SpriteBase3D.ALPHA_CUT_DISCARD
		s.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
		s.shaded = false
		s.double_sided = true
		s.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		add_child(s)
		sprites.append(s)
		var b := MeshInstance3D.new()
		var q := QuadMesh.new()
		q.size = Vector2(0.8, 0.45)
		q.orientation = PlaneMesh.FACE_Y
		b.mesh = q
		b.material_override = blob_mat
		b.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(b)
		blobs.append(b)
	for i in sprites.size():
		var s: Sprite3D = sprites[i]
		var b: MeshInstance3D = blobs[i]
		if i >= n:
			s.visible = false
			b.visible = false
			continue
		var x := data[i * 7]
		var y := data[i * 7 + 1]
		var w := data[i * 7 + 2]
		var h := data[i * 7 + 3]
		var flags := data[i * 7 + 6]
		s.region_rect = Rect2(data[i * 7 + 4], data[i * 7 + 5], w, h)
		s.modulate = Color(1, 1, 1, 0.6 if flags & 1 else 1.0)
		if flags & 2:
			# movement arrow: flat on the ground
			var cx := (x + w * 0.5) * PX
			var cz := (y + h * 0.5) * PX
			s.billboard = BaseMaterial3D.BILLBOARD_DISABLED
			s.axis = Vector3.AXIS_Y
			s.offset = Vector2.ZERO
			s.scale = Vector3.ONE
			s.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			s.position = Vector3(cx, height_at(cx, cz) + 0.04, cz)
			s.rotation = Vector3.ZERO
			s.visible = true
			b.visible = false
			continue
		s.billboard = _stand_mode()
		s.axis = Vector3.AXIS_Z
		s.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		s.offset = Vector2(0, h * 0.5)
		var wx := (x + w * 0.5) * PX
		var wz := (y + h) * PX - 0.5
		var bf: float = bld_front.get(_cell_index(int(floor(wx)) + MARGIN, int(floor(wz)) + MARGIN), -INF)
		if bf > -INF:
			wz = maxf(wz, bf + 0.08)
		var gy := height_at(wx, wz)
		s.position = Vector3(wx, gy, wz)
		s.scale = Vector3(1, 1.18, 1)
		s.visible = true
		b.position = Vector3(wx, gy + 0.02, wz)
		b.visible = true

func _update_range() -> void:
	var cells: PackedByteArray = core.cells()
	if cells == last_cells:
		return
	last_cells = cells
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var any := false
	for my in map_h:
		for mx in map_w:
			var i := my * map_w + mx
			if i >= cells.size() or cells[i] == 0:
				continue
			any = true
			var col := Color(0.15, 0.4, 1.0, 0.55) if cells[i] == 1 else Color(1.0, 0.2, 0.15, 0.55)
			var pts := []
			for c in [Vector2(0, 0), Vector2(1, 0), Vector2(1, 1), Vector2(0, 1)]:
				var cv: Vector2 = c
				var hh := height_at(mx + 0.02 + cv.x * 0.96, my + 0.02 + cv.y * 0.96)
				pts.append([Vector3(mx + cv.x, hh + 0.03, my + cv.y), cv])
			for k in [0, 1, 2, 0, 2, 3]:
				st.set_color(col)
				st.set_uv(pts[k][1])
				st.add_vertex(pts[k][0])
	if not any:
		range_mi.visible = false
		return
	range_mi.visible = true
	range_mi.mesh = st.commit()
	if range_mi.material_override == null:
		var m := ShaderMaterial.new()
		m.shader = load("res://shaders/range.gdshader")
		m.render_priority = 1
		range_mi.material_override = m

func _update_cursor(delta: float) -> void:
	var c: Vector3i = core.cursor()
	cursor_mi.visible = c.z != 0
	if c.z == 0:
		return
	var tx := c.x * PX + 0.5
	var tz := c.y * PX + 0.5
	var target := Vector3(tx, height_at(tx, tz) + 0.05, tz)
	if cursor_pos.distance_to(target) > 4.0:
		cursor_pos = target
	cursor_pos = cursor_pos.lerp(target, 1.0 - exp(-delta * 25.0))
	cursor_mi.position = cursor_pos

func _update_camera(delta: float) -> void:
	var v: Vector2i = core.view()
	var t := Vector3((v.x + 120) * PX, 0.0, (v.y + 80) * PX) + pan
	t.y = height_at(t.x, t.z) * 0.5
	if not cam_ready:
		cam_target = t
		cam_ready = true
	cam_target = cam_target.lerp(t, 1.0 - exp(-delta * 8.0))
	var k := 1.0 - exp(-delta * 12.0)
	_zoom_s = lerpf(_zoom_s, zoom, k)
	_yaw_s = lerpf(_yaw_s, yaw, k)
	_pitch_s = lerpf(_pitch_s, pitch_deg, k)
	var pitch := deg_to_rad(_pitch_s)
	var dist := 15.5 * _zoom_s
	var back := Vector3(sin(deg_to_rad(_yaw_s)), 0, cos(deg_to_rad(_yaw_s)))
	var pos := cam_target + back * cos(pitch) * dist + Vector3(0, sin(pitch) * dist, 0)
	cam.position = pos
	cam.look_at(cam_target, Vector3.UP)
	_update_standee_mode()

## Mouse -> map pixel (or null if the ray misses the ground).
func pick(screen_pos: Vector2):
	var o := cam.project_ray_origin(screen_pos)
	var d := cam.project_ray_normal(screen_pos)
	if absf(d.y) < 1e-4:
		return null
	var h := 0.0
	var p := Vector3.ZERO
	for i in 4:
		var t := (h - o.y) / d.y
		if t < 0:
			return null
		p = o + d * t
		h = height_at(p.x, p.z)
	return Vector2(p.x * 16.0, p.z * 16.0)
