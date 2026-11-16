## Fire Emblem HD-2D demo: own main menu, game in 2D/HD-2D, pause menu.
extends Node
const Ui = preload("res://scripts/ui.gd")

const GBA_A := 1
const GBA_B := 2
const GBA_SELECT := 4
const GBA_START := 8
const GBA_RIGHT := 16
const GBA_LEFT := 32
const GBA_UP := 64
const GBA_DOWN := 128
const GBA_R := 256
const GBA_L := 512
const FPS := 59.7275

# PC layout (no GBA buttons): Enter/Space confirm, Backspace back (Esc too, see _input),
# Q/Tab next unit, E/I unit info, M map overview / skip scene, C extra (Select).
const KEYMAP := {
	KEY_UP: GBA_UP, KEY_W: GBA_UP, KEY_DOWN: GBA_DOWN, KEY_S: GBA_DOWN,
	KEY_LEFT: GBA_LEFT, KEY_A: GBA_LEFT, KEY_RIGHT: GBA_RIGHT, KEY_D: GBA_RIGHT,
	KEY_ENTER: GBA_A, KEY_KP_ENTER: GBA_A, KEY_SPACE: GBA_A,
	KEY_BACKSPACE: GBA_B,
	KEY_Q: GBA_L, KEY_TAB: GBA_L, KEY_E: GBA_R, KEY_I: GBA_R,
	KEY_M: GBA_START, KEY_C: GBA_SELECT,
}
const PADMAP := {
	JOY_BUTTON_A: GBA_A, JOY_BUTTON_B: GBA_B, JOY_BUTTON_BACK: GBA_SELECT, JOY_BUTTON_START: GBA_START,
	JOY_BUTTON_DPAD_UP: GBA_UP, JOY_BUTTON_DPAD_DOWN: GBA_DOWN, JOY_BUTTON_DPAD_LEFT: GBA_LEFT,
	JOY_BUTTON_DPAD_RIGHT: GBA_RIGHT, JOY_BUTTON_LEFT_SHOULDER: GBA_L, JOY_BUTTON_RIGHT_SHOULDER: GBA_R,
}

enum Mode { ERROR, MENU, SLOTS, GAME, LAUNCH }

var core
var mode := Mode.MENU
var world
var acc := 0.0
var paused := false
var cfg := ConfigFile.new()
var settings := {"fullscreen": false, "volume": 0.8, "dof": true, "shadows": true, "hd": true, "talk": true, "menus": true,
	"lang": "en", "quality": 2, "fps": false, "instant": false, "ffwd": 4, "rom": "", "debug": false}
var fps_label: Label
var toast: Label
var toast_t := 0.0
const QUICK := "user://quicksave.st"

# ---------------------------------------------------------------- language
# The game itself is English; the own screens follow the language setting
# (texts are written in German and translated here).
const EN := {
	"Dieses Remake enthält keine Spieldaten. Es braucht dein eigenes ROM von\nFire Emblem (GBA, USA) – zum Beispiel eine Sicherungskopie deines Moduls.": "This remake contains no game data. It needs your own ROM of\nFire Emblem (GBA, USA) - for example a backup of your cartridge.",
	"Inoffizielles HD-2D-Remake": "Unofficial HD-2D remake",
	"Debug-Modus (Kapitel wählen)": "Debug mode (choose chapter)", "Kapitel wählen (Debug)": "Choose Chapter (Debug)",
	"Kapitel wählen": "Choose Chapter", "Lyns Geschichte": "Lyn's Tale", "Eliwoods Geschichte": "Eliwood's Tale", "Hectors Geschichte": "Hector's Tale",
	"Start mit": "Start with", "Einheiten aus einem Spielstand": "Units from a save file", "Neues Spiel (nur Einheiten des Kapitels)": "New game (chapter's units only)",
	"Der Spielstand wird zu Kapitelbeginn überschrieben. Der Story-Modus (Lyn/Eliwood/Hector) bleibt der des Spielstands.": "The save file is overwritten when the chapter starts. The story mode (Lyn/Eliwood/Hector) stays that of the save file.", "Spielen": "Play", "ROM auswählen …": "Choose ROM …", "ROM auswählen": "Choose ROM",
	"Tipp: Du kannst die ROM-Datei auch einfach in dieses Fenster ziehen.": "Tip: you can also drop the ROM file onto this window.",
	"Noch kein ROM gewählt.": "No ROM chosen yet.", "ROM gefunden und geprüft:": "ROM found and checked:",
	"Datei nicht lesbar.": "File cannot be read.", "Keine GBA-ROM-Datei.": "Not a GBA ROM file.",
	"Das ist die europäische Version. Benötigt wird Fire Emblem (USA).": "This is the European version. Fire Emblem (USA) is needed.",
	"Das ist die japanische Version. Benötigt wird Fire Emblem (USA).": "This is the Japanese version. Fire Emblem (USA) is needed.",
	"Das ist kein Fire-Emblem-ROM (USA).": "This is not a Fire Emblem (USA) ROM.",
	"Fire Emblem (USA), aber verändert (Patch/Hack?). Benötigt wird ein unverändertes ROM.": "Fire Emblem (USA), but modified (patch/hack?). An unmodified ROM is needed.",
	"Befehlshilfe": "Command Help",
	"Kartenübersicht": "Map Overview",
	"Hilfe": "Help",
	"Menü": "Menu",
	"Truppe wählen": "Pick Units", "Tauschen": "Trade", "Noch %d Einheiten wählen": "Pick %d more units",
	"Truppe vollständig": "Party complete", "Einheit wählen": "Choose a unit", "%s tauscht mit …": "%s trades with …",
	"Einheitenliste": "Unit List",
	"Spielstand %d überschreiben?": "Overwrite save %d?",
	"Vorbereitung": "Preparations", "Ziel": "Goal", "Kriegskasse": "Funds", "Zum Kampf!": "Fight!",
	"Januar": "January", "Februar": "February", "März": "March", "April": "April", "Mai": "May", "Juni": "June",
	"Juli": "July", "August": "August", "September": "September", "Oktober": "October", "November": "November", "Dezember": "December",
	"Mär": "Mar", "Okt": "Oct", "Dez": "Dec",
	"Die Spiel-Erweiterung (bin/libfe7hd.so) konnte nicht geladen werden.": "The game extension (bin/libfe7hd.so) could not be loaded.",
	"\nLege das US-ROM (Fire Emblem, .gba) in den Ordner neben fe7-hd2d oder darüber.": "\nPut the US ROM (Fire Emblem, .gba) into the folder next to fe7-hd2d or above it.",
	"Maus oder Pfeiltasten + Enter  ·  Esc zurück  ·  F11 Vollbild": "Mouse or arrow keys + Enter  ·  Esc back  ·  F11 fullscreen",
	"Kapitel fortsetzen": "Resume Chapter", "Kapitel laden": "Restart Chapter", "Spielstand löschen": "Erase Data", "Neues Spiel": "New Game",
	"Leerer Spielstand": "Empty file", "Spielzeit": "Play time", "Noch kein Spiel in diesem Platz": "No game in this slot yet",
	"Achtung: Dieser Spielstand wird überschrieben.": "Warning: this file will be overwritten.",
	"Spielstand speichern": "Save Game", "In welchem Platz soll gespeichert werden?": "Which file do you want to save to?",
	"Zurück": "Back", "Spielstand": "Save File",
	"Welchen Spielstand fortsetzen?": "Which file do you want to resume?", "Welches Kapitel laden?": "Which chapter do you want to restart?",
	"Welchen Spielstand löschen?": "Which file do you want to erase?",
	"In welchem Platz soll das neue Spiel gespeichert werden?": "Which slot should the new game be saved in?",
	"Achtung: Spielstand %d wird gelöscht und überschrieben!\nTrotzdem ein neues Spiel beginnen?": "Warning: file %d will be erased and overwritten!\nStart a new game anyway?",
	"Spielstand %d wirklich löschen?": "Really erase file %d?",
	"Einen Moment …": "One moment …", "Ja": "Yes", "Nein": "No", "Erledigt.": "Done.",
	"Persönliche Daten": "Personal Data", "name": "name", "geburtsmonat": "birth month", "geschlecht": "gender",
	"Männlich": "Male", "Weiblich": "Female",
	"Eintrag anklicken oder Pfeiltasten + Enter  ·  Namen einfach tippen  ·  Esc schließt Auswahl": "Click an entry or use arrow keys + Enter  ·  Just type the name  ·  Esc closes a list",
	"Lege Namen, Geburtsmonat und Geschlecht fest.": "Choose your name, birth month and gender.",
	"▶ Fertig": "▶ Done", "Sind das die gewünschten Angaben?": "Is this information correct?",
	"Fortsetzen": "Resume", "Einstellungen": "Settings", "Beenden": "Quit",
	"Vollbild": "Fullscreen", "HD-2D-Karte (aus = Originalgrafik)": "HD-2D map (off = original graphics)",
	"Tiefenunschärfe": "Depth of field", "Schatten": "Shadows", "Sprechblasen neu gezeichnet": "Redrawn speech bubbles",
	"Menüs neu gezeichnet": "Redrawn menus", "Text sofort ganz zeigen": "Show each text page at once",
	"FPS anzeigen": "Show FPS", "Lautstärke": "Volume", "Sprache": "Language", "Grafik": "Graphics",
	"Niedrig": "Low", "Mittel": "Medium", "Hoch": "High", "Vorspulen (Strg)": "Fast forward (Ctrl)",
	"Pause": "Pause", "Weiterspielen": "Continue", "Hauptmenü": "Main Menu",
	"Schnellspeichern (F5)": "Quick save (F5)", "Schnellladen (F8)": "Quick load (F8)",
	"Schnellgespeichert": "Quick saved", "Schnellstand geladen": "Quick save loaded", "Kein Schnellstand vorhanden": "No quick save yet",
	"Lade …": "Loading …",
	"STEUERUNG_HILFE": "Controls: arrows/WASD move · Enter/Space confirm · Esc/Backspace back · P/F1 pause\nQ/Tab next unit · E/I unit info · M map overview / skip scene · Ctrl hold = fast forward\nF5 quick save · F8 quick load · F11 fullscreen\nCamera: wheel on the map or +/- zoom · middle mouse drag or Z/X rotate, R/F tilt\nShift + middle mouse drag moves the camera · Home resets it\nMouse: click = select · right click = back · wheel in menus = scroll",
}
const DE_HELP := "Steuerung: Pfeile/WASD bewegen · Enter/Leertaste bestätigen · Esc/Rücktaste zurück · P/F1 Pause\nQ/Tab nächste Einheit · E/I Einheiteninfo · M Kartenübersicht / Szene überspringen · Strg halten = Vorspulen\nF5 Schnellspeichern · F8 Schnellladen · F11 Vollbild\nKamera: Mausrad auf der Karte oder +/- zoomen · mittlere Maustaste ziehen oder Z/X drehen, R/F neigen\nShift + mittlere Maustaste ziehen verschiebt die Kamera · Pos1 setzt sie zurück\nMaus: anklicken = wählen · Rechtsklick = zurück · Rad in Menüs = blättern"
var _tr_nodes: Array = []  # [node, german text]

## Folder for test screenshots and dumps (FE7HD_SHOTS, else user://shots).
func shot_dir() -> String:
	var d := OS.get_environment("FE7HD_SHOTS")
	return d if d != "" else OS.get_user_data_dir() + "/shots"

func T(de: String) -> String:
	if settings.get("lang", "en") == "de":
		return DE_HELP if de == "STEUERUNG_HILFE" else de
	return EN.get(de, de)

## Sets a node's text and keeps it for language changes.
func _tt(node: Object, de: String) -> void:
	node.text = T(de)
	_tr_nodes.append([node, de])

func _retranslate() -> void:
	var keep: Array = []
	for e in _tr_nodes:
		if is_instance_valid(e[0]):
			e[0].text = T(e[1])
			keep.append(e)
	_tr_nodes = keep
	if lang_opt:
		lang_opt.select(1 if settings.lang == "de" else 0)
	for e in _tr_opts:
		for k in e[1].size():
			e[0].set_item_text(k, T(e[1][k]))
	if p_month_btn:
		_set_month(p_month_idx)
		_set_gender(p_gender_idx)
	if mode == Mode.MENU and menu_built_mask >= 0:
		_rebuild_menu_items(menu_built_mask)
	if mode == Mode.SLOTS and slots_built and not picking:
		_open_slots_titles()
		_show_slots()
var talk_ui: Control
var menu_ui: Control
var mapinfo_ui: Control
var battle_ui: Control
var levelup_ui: Control
var banner_ui: Control
var trade_ui: Control
var shop_ui: Control
var narr_ui: Control
var prepbar_ui: Control
var minimap_ui: Control
var shop_face_frames := 0
var lv_face_frames := 0
var status_root: Control
var status_ui: Control
var unitlist_ui: Control
var prep_ui: Control
var status_bg: TextureRect
var status_bg_on := false
var shop_bg_on := false
var prepunits_ui: Control
var options_ui: Control

# nodes
var screen_rect: TextureRect
var screen_tex := ImageTexture.new()
var screen_img: Image
var fade_rect: ColorRect
var tilt_rect: ColorRect
var ui_layer: CanvasLayer
var menu_root: Control
var menu_bg: TextureRect
var menu_box: VBoxContainer
var menu_status: Label
var title_box: VBoxContainer
var slot_panel: VBoxContainer
var slot_title: Label
var slot_cards: VBoxContainer
var slot_bit := 4
var slots_built := false
var picking := false
var personal_root: Control
var p_name: LineEdit
var p_ok: Button
var personal_state := 0  # 0 none, 1 form shown, 2 applying
const MONTHS := ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"]
var settings_panel: PanelContainer
var pause_panel: PanelContainer
var player: AudioStreamPlayer
var playback: AudioStreamGeneratorPlayback
var font_title: Font
var mouse_left := false
var mouse_right := false
var mouse_middle := false
var wheel := 0
var menu_built_mask := -1
var gold := Color(0.93, 0.8, 0.45)
# automated test: FE7HD_TEST="frame:cmd:arg;..." (cmd: shot, keys, choose, load, mouse, click, rclick, quit)
var test_cmds := []
var test_frame := 0
var test_keys := 0
var test_keys_until := 0
var test_mouse = null

func _ready() -> void:
	_load_settings()
	world = preload("res://scripts/world.gd").new()
	add_child(world)
	world.visible = false

	# layers: 0 = fade over the 3D world, 1 = game picture/interface, 2 = menus
	var fl := CanvasLayer.new()
	fl.layer = 0
	add_child(fl)
	tilt_rect = ColorRect.new()
	tilt_rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	tilt_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var tm := ShaderMaterial.new()
	tm.shader = load("res://shaders/tiltshift.gdshader")
	tilt_rect.material = tm
	tilt_rect.visible = false
	fl.add_child(tilt_rect)
	fade_rect = ColorRect.new()
	fade_rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	fade_rect.color = Color(0, 0, 0, 0)
	fade_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	fl.add_child(fade_rect)

	var gl := CanvasLayer.new()
	gl.layer = 1
	add_child(gl)
	screen_rect = TextureRect.new()
	screen_rect.texture = screen_tex
	screen_rect.stretch_mode = TextureRect.STRETCH_SCALE
	screen_rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	screen_rect.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	screen_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var sm := ShaderMaterial.new()
	sm.shader = load("res://shaders/ui_sharp.gdshader")
	screen_rect.material = sm
	gl.add_child(screen_rect)

	ui_layer = CanvasLayer.new()
	ui_layer.layer = 2
	add_child(ui_layer)

	# the game's menu font for every plain label and button
	var th := Theme.new()
	th.default_font = f_ui
	th.default_font_size = 22
	ui_theme = th
	font_title = f_ui   # headings and buttons use the game's menu font too

	player = AudioStreamPlayer.new()
	var gen := AudioStreamGenerator.new()
	gen.mix_rate = 32768
	gen.buffer_length = 0.12
	player.stream = gen
	add_child(player)
	player.play()
	playback = player.get_stream_playback()
	_apply_volume()

	_build_menu()
	_build_slots()
	_build_personal()
	mapinfo_ui = preload("res://scripts/mapinfo.gd").new()
	mapinfo_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	screen_rect.add_child(mapinfo_ui)
	mapinfo_ui.font = f_narrow
	mapinfo_ui.ink_font = f_talk
	mapinfo_ui.visible = false
	battle_ui = preload("res://scripts/battle.gd").new()
	battle_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	screen_rect.add_child(battle_ui)
	battle_ui.font = f_narrow
	battle_ui.ink_font = f_talk
	battle_ui.visible = false
	levelup_ui = preload("res://scripts/levelup.gd").new()
	levelup_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	screen_rect.add_child(levelup_ui)
	levelup_ui.font = f_narrow
	levelup_ui.ink_font = f_talk
	levelup_ui.title_font = f_talk
	levelup_ui.visible = false
	trade_ui = preload("res://scripts/trade.gd").new()
	trade_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	screen_rect.add_child(trade_ui)
	trade_ui.font = f_narrow
	trade_ui.ink_font = f_talk
	trade_ui.title_font = f_talk
	trade_ui.icon_cb = func(n: int) -> Texture2D:
		var im: Image = core.item_icon(n)
		return ImageTexture.create_from_image(im) if im else null
	trade_ui.visible = false
	shop_ui = preload("res://scripts/shop.gd").new()
	shop_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	screen_rect.add_child(shop_ui)
	shop_ui.font = f_narrow
	shop_ui.ink_font = f_talk
	shop_ui.title_font = f_talk
	shop_ui.icon_cb = trade_ui.icon_cb
	shop_ui.visible = false
	narr_ui = preload("res://scripts/narration.gd").new()
	narr_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	screen_rect.add_child(narr_ui)
	narr_ui.font = f_narrow
	narr_ui.visible = false
	banner_ui = preload("res://scripts/banner.gd").new()
	banner_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	screen_rect.add_child(banner_ui)
	banner_ui.title_font = f_talk
	banner_ui.visible = false
	menu_ui = preload("res://scripts/menu.gd").new()
	menu_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	screen_rect.add_child(menu_ui)
	menu_ui.font = f_narrow
	menu_ui.ink_font = f_talk
	menu_ui.visible = false
	talk_ui = preload("res://scripts/talk.gd").new()
	talk_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	screen_rect.add_child(talk_ui)
	talk_ui.set_font(f_talk)
	talk_ui.visible = false
	_build_status()
	_build_settings()
	_build_pause()
	_apply_quality()
	world.apply_settings(settings)

	core = ClassDB.instantiate("Fe7Core")
	if core == null:
		_show_error(T("Die Spiel-Erweiterung (bin/libfe7hd.so) konnte nicht geladen werden."))
		return
	core.set_hd(settings.hd)
	core.set_talk_native(settings.talk)
	core.set_menu_native(settings.menus)
	core.set_instant_text(settings.instant)
	get_viewport().size_changed.connect(_layout)
	# the remake carries no game data: the player's own ROM is needed
	var rom: String = str(settings.rom)
	var t0 := OS.get_environment("FE7HD_TEST")
	if OS.get_environment("FE7HD_LAUNCHER") != "":
		rom = ""
		for c in t0.split(";", false):
			test_cmds.append(c.split(":"))
	elif (rom == "" or not core.check_rom(rom).ok) and t0 != "":
		rom = core.find_rom(ProjectSettings.globalize_path("res://"))
	if rom == "" or not core.check_rom(rom).ok:
		if t0 != "" and test_cmds.is_empty():
			for c in t0.split(";", false):
				test_cmds.append(c.split(":"))
		_open_launcher(core.find_rom(ProjectSettings.globalize_path("res://")))
		return
	_boot(rom)

## Starts the game from the chosen ROM (after the launcher or directly).
func _boot(rom: String) -> void:
	var err: String = core.start(rom)
	if err != "":
		_open_launcher("", err)
		return
	Ui.core = core
	Ui.refresh()
	_fill_font(f_px, 0)
	_fill_font(f_talk, 1)
	_fill_font(f_ui, 0, 40)
	# canvas layers cut the theme chain: hand the theme to every top control
	for c in ui_layer.get_children():
		if c is Control:
			(c as Control).theme = ui_theme
	_enter_menu()
	var t := OS.get_environment("FE7HD_TEST")
	if t != "" and test_frame == 0:
		for c in t.split(";", false):
			test_cmds.append(c.split(":"))
	_layout()

# ---------------------------------------------------------------- launcher

var launch_root: Control
var launch_path := ""
var launch_info: Label
var launch_play: Button
var launch_dialog: FileDialog

func _launch_button(text: String, cb: Callable, main := false) -> Button:
	var b := Button.new()
	b.text = text
	b.custom_minimum_size = Vector2(620, 92)
	b.add_theme_font_override("font", f_smooth)
	b.add_theme_font_size_override("font_size", 44)
	for st in ["normal", "hover", "focus", "pressed", "disabled"]:
		var sb := StyleBoxFlat.new()
		var base := Color(0.16, 0.24, 0.62) if main else Color(0.13, 0.17, 0.36)
		if st in ["hover", "focus", "pressed"]:
			base = base.lightened(0.25)
		if st == "disabled":
			base = Color(0.2, 0.2, 0.26)
		sb.bg_color = base
		sb.border_color = Color(0.93, 0.88, 0.66) if st != "disabled" else Color(0.4, 0.4, 0.45)
		sb.set_border_width_all(3)
		sb.set_corner_radius_all(6)
		b.add_theme_stylebox_override(st, sb)
	b.add_theme_color_override("font_color", Color(0.95, 0.95, 1.0))
	b.add_theme_color_override("font_disabled_color", Color(0.55, 0.55, 0.6))
	b.pressed.connect(cb)
	b.mouse_entered.connect(func(): if not b.disabled: b.grab_focus())
	return b

func _open_launcher(found: String, err := "") -> void:
	mode = Mode.LAUNCH
	menu_root.visible = false
	if launch_root == null:
		launch_root = Control.new()
		launch_root.set_anchors_preset(Control.PRESET_FULL_RECT)
		ui_layer.add_child(launch_root)
		var bg := ColorRect.new()
		bg.set_anchors_preset(Control.PRESET_FULL_RECT)
		bg.material = ShaderMaterial.new()
		(bg.material as ShaderMaterial).shader = load("res://shaders/marble.gdshader")
		launch_root.add_child(bg)
		var shade := ColorRect.new()
		shade.set_anchors_preset(Control.PRESET_FULL_RECT)
		shade.color = Color(0.02, 0.03, 0.08, 0.55)
		launch_root.add_child(shade)
		var box := VBoxContainer.new()
		box.set_anchors_preset(Control.PRESET_CENTER)
		box.grow_horizontal = Control.GROW_DIRECTION_BOTH
		box.grow_vertical = Control.GROW_DIRECTION_BOTH
		box.alignment = BoxContainer.ALIGNMENT_CENTER
		box.add_theme_constant_override("separation", 18)
		launch_root.add_child(box)
		var title := Label.new()
		title.text = "FE7-HD2D"
		title.add_theme_font_override("font", f_smooth)
		title.add_theme_font_size_override("font_size", 130)
		title.add_theme_color_override("font_outline_color", Color(0.2, 0.12, 0.04))
		title.add_theme_constant_override("outline_size", 10)
		title.add_theme_color_override("font_color", Color(0.98, 0.9, 0.6))
		title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		box.add_child(title)
		var sub := Label.new()
		sub.text = T("Dieses Remake enthält keine Spieldaten. Es braucht dein eigenes ROM von\nFire Emblem (GBA, USA) – zum Beispiel eine Sicherungskopie deines Moduls.")
		sub.add_theme_font_override("font", f_smooth)
		sub.add_theme_font_size_override("font_size", 38)
		sub.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		box.add_child(sub)
		launch_info = Label.new()
		launch_info.add_theme_font_override("font", f_smooth)
		launch_info.add_theme_font_size_override("font_size", 34)
		launch_info.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		launch_info.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		launch_info.custom_minimum_size = Vector2(1400, 130)
		box.add_child(launch_info)
		var col := VBoxContainer.new()
		col.alignment = BoxContainer.ALIGNMENT_CENTER
		col.add_theme_constant_override("separation", 12)
		col.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
		box.add_child(col)
		launch_play = _launch_button(T("Spielen"), _launch_start, true)
		col.add_child(launch_play)
		col.add_child(_launch_button(T("ROM auswählen …"), _launch_pick))
		col.add_child(_launch_button(T("Beenden"), func(): get_tree().quit()))
		var hint := Label.new()
		hint.text = T("Tipp: Du kannst die ROM-Datei auch einfach in dieses Fenster ziehen.")
		hint.add_theme_font_override("font", f_smooth)
		hint.add_theme_font_size_override("font_size", 30)
		hint.add_theme_color_override("font_color", Color(0.75, 0.78, 0.9))
		hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		box.add_child(hint)
		launch_dialog = FileDialog.new()
		launch_dialog.access = FileDialog.ACCESS_FILESYSTEM
		launch_dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
		launch_dialog.filters = PackedStringArray(["*.gba ; GBA-ROM"])
		launch_dialog.title = T("ROM auswählen")
		launch_dialog.size = Vector2i(1100, 700)
		launch_dialog.file_selected.connect(_launch_set)
		launch_root.add_child(launch_dialog)
		get_window().files_dropped.connect(func(files: PackedStringArray):
			if mode == Mode.LAUNCH and files.size() > 0:
				_launch_set(files[0]))
	launch_root.visible = true
	_launch_set(found if found != "" else str(settings.rom), err)

func _launch_set(path: String, err := "") -> void:
	launch_path = ""
	var msg := ""
	if path == "":
		msg = T("Noch kein ROM gewählt.")
	else:
		var r: Dictionary = core.check_rom(path)
		if r.ok:
			launch_path = path
			msg = T("ROM gefunden und geprüft:") + "\n" + path
		else:
			msg = path.get_file() + ": " + T(str(r.msg))
	if err != "":
		msg = err + "\n" + msg
	launch_info.text = msg
	launch_info.add_theme_color_override("font_color", Color(0.6, 1.0, 0.65) if launch_path != "" else Color(1.0, 0.72, 0.62))
	launch_play.disabled = launch_path == ""
	if launch_path != "":
		launch_play.grab_focus()

func _launch_pick() -> void:
	if str(settings.rom) != "":
		launch_dialog.current_dir = str(settings.rom).get_base_dir()
	launch_dialog.popup_centered()

func _launch_start() -> void:
	if launch_path == "":
		return
	settings.rom = launch_path
	_save_settings()
	launch_root.visible = false
	_boot(launch_path)

# ---------------------------------------------------------------- settings

func _load_settings() -> void:
	if cfg.load("user://settings.cfg") == OK:
		for k in settings.keys():
			settings[k] = cfg.get_value("hd2d", k, settings[k])
	if settings.fullscreen:
		DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN)

func _save_settings() -> void:
	for k in settings.keys():
		cfg.set_value("hd2d", k, settings[k])
	cfg.save("user://settings.cfg")

func _apply_quality() -> void:
	var q: int = int(settings.quality)
	var vp := get_viewport()
	vp.msaa_3d = [Viewport.MSAA_DISABLED, Viewport.MSAA_2X, Viewport.MSAA_4X][clampi(q, 0, 2)]
	vp.scaling_3d_scale = 0.67 if q == 0 else 1.0
	if fps_label:
		fps_label.visible = settings.fps

func _apply_volume() -> void:
	var v: float = settings.volume
	player.volume_db = linear_to_db(max(v, 0.0001))

func _apply_all() -> void:
	if core:
		core.set_hd(settings.hd)
		core.set_talk_native(settings.talk)
		core.set_menu_native(settings.menus)
		core.set_instant_text(settings.instant)
	_apply_quality()
	DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN if settings.fullscreen else DisplayServer.WINDOW_MODE_WINDOWED)
	_apply_volume()
	world.apply_settings(settings)
	_save_settings()

# ---------------------------------------------------------------- UI building

func _panel_style(_alpha := 0.82) -> StyleBox:
	# the game's blue window
	var sb := Ui.stylebox("win", 4)
	sb.axis_stretch_vertical = StyleBoxTexture.AXIS_STRETCH_MODE_TILE
	sb.content_margin_left = 44
	sb.content_margin_right = 44
	sb.content_margin_top = 34
	sb.content_margin_bottom = 36
	return sb

func _button(text: String, cb: Callable) -> Button:
	var b := Button.new()
	b.text = text
	b.custom_minimum_size = Vector2(460, 69)
	b.add_theme_font_size_override("font_size", 31)
	b.add_theme_font_override("font", font_title)
	# the bars of the game's own main menu
	var n := Ui.stylebox("bar", 3)
	var h := Ui.stylebox("barsel", 3)
	b.add_theme_stylebox_override("normal", n)
	b.add_theme_stylebox_override("hover", h)
	b.add_theme_stylebox_override("focus", h)
	b.add_theme_stylebox_override("pressed", h)
	b.add_theme_color_override("font_color", Color(0.92, 0.92, 0.96))
	b.add_theme_color_override("font_hover_color", Color(1, 0.95, 0.75))
	b.add_theme_color_override("font_focus_color", Color(1, 0.95, 0.75))
	b.pressed.connect(cb)
	b.mouse_entered.connect(func(): b.grab_focus())
	return b

func _build_menu() -> void:
	menu_root = Control.new()
	menu_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	ui_layer.add_child(menu_root)
	menu_bg = TextureRect.new()
	menu_bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	menu_bg.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	menu_bg.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	menu_bg.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	menu_bg.modulate = Color(0.55, 0.6, 0.75)
	menu_root.add_child(menu_bg)
	var shade := ColorRect.new()
	shade.set_anchors_preset(Control.PRESET_FULL_RECT)
	shade.color = Color(0.02, 0.03, 0.08, 0.45)
	menu_root.add_child(shade)
	# floating light motes
	var motes := CPUParticles2D.new()
	motes.amount = 70
	motes.lifetime = 9.0
	motes.preprocess = 9.0
	motes.emission_shape = CPUParticles2D.EMISSION_SHAPE_RECTANGLE
	motes.emission_rect_extents = Vector2(1000, 600)
	motes.direction = Vector2(0, -1)
	motes.spread = 25
	motes.gravity = Vector2.ZERO
	motes.initial_velocity_min = 8
	motes.initial_velocity_max = 30
	motes.scale_amount_min = 1.5
	motes.scale_amount_max = 4.0
	motes.color = Color(1.0, 0.88, 0.55, 0.55)
	motes.name = "Motes"
	menu_root.add_child(motes)

	var center := VBoxContainer.new()
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	center.alignment = BoxContainer.ALIGNMENT_CENTER
	center.add_theme_constant_override("separation", 10)
	menu_root.add_child(center)
	var title := Label.new()
	title.text = "FE7-HD2D"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_override("font", font_title)
	title.add_theme_font_size_override("font_size", 92)
	title.add_theme_color_override("font_color", Color(0.98, 0.9, 0.62))
	title.add_theme_color_override("font_outline_color", Color(0.25, 0.12, 0.02))
	title.add_theme_constant_override("outline_size", 14)
	title.add_theme_color_override("font_shadow_color", Color(1, 0.7, 0.2, 0.45))
	title.add_theme_constant_override("shadow_offset_y", 0)
	title.add_theme_constant_override("shadow_outline_size", 34)
	center.add_child(title)
	var sub := Label.new()
	_tt(sub, "Inoffizielles HD-2D-Remake")
	sub.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	sub.add_theme_font_override("font", font_title)
	sub.add_theme_font_size_override("font_size", 30)
	sub.add_theme_color_override("font_color", Color(0.8, 0.86, 1.0))
	center.add_child(sub)
	var gap := Control.new()
	gap.custom_minimum_size = Vector2(0, 40)
	center.add_child(gap)
	var hb := HBoxContainer.new()
	hb.alignment = BoxContainer.ALIGNMENT_CENTER
	center.add_child(hb)
	menu_box = VBoxContainer.new()
	menu_box.add_theme_constant_override("separation", 14)
	hb.add_child(menu_box)
	menu_status = Label.new()
	menu_status.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	menu_status.add_theme_font_size_override("font_size", 24)
	menu_status.add_theme_color_override("font_color", Color(0.8, 0.85, 1.0, 0.8))
	center.add_child(menu_status)
	var foot := Label.new()
	_tt(foot, "Maus oder Pfeiltasten + Enter  ·  Esc zurück  ·  F11 Vollbild")
	foot.add_theme_font_size_override("font_size", 20)
	foot.add_theme_color_override("font_color", Color(0.75, 0.8, 0.95, 0.6))
	foot.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	foot.position.y -= 40
	foot.grow_horizontal = Control.GROW_DIRECTION_BOTH
	foot.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	menu_root.add_child(foot)

const SLOT_TITLES := {0: "Kapitel fortsetzen", 1: "Kapitel laden", 3: "Spielstand löschen", 4: "Neues Spiel", 9: "Spielstand speichern"}

func _build_slots() -> void:
	slot_panel = VBoxContainer.new()
	slot_panel.alignment = BoxContainer.ALIGNMENT_CENTER
	slot_panel.add_theme_constant_override("separation", 18)
	slot_panel.set_anchors_preset(Control.PRESET_FULL_RECT)
	slot_panel.visible = false
	menu_root.add_child(slot_panel)
	slot_title = Label.new()
	slot_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	slot_title.add_theme_font_override("font", font_title)
	slot_title.add_theme_font_size_override("font_size", 52)
	slot_title.add_theme_color_override("font_color", Color(0.98, 0.9, 0.62))
	slot_title.add_theme_color_override("font_outline_color", Color(0.25, 0.12, 0.02))
	slot_title.add_theme_constant_override("outline_size", 10)
	slot_panel.add_child(slot_title)
	var sub := Label.new()
	sub.name = "Sub"
	sub.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	sub.add_theme_font_size_override("font_size", 24)
	sub.add_theme_color_override("font_color", Color(0.8, 0.86, 1.0, 0.85))
	slot_panel.add_child(sub)
	var hb := HBoxContainer.new()
	hb.alignment = BoxContainer.ALIGNMENT_CENTER
	slot_panel.add_child(hb)
	slot_cards = VBoxContainer.new()
	slot_cards.add_theme_constant_override("separation", 16)
	hb.add_child(slot_cards)

func _fmt_time(frames: int) -> String:
	var sec := frames / 60
	return "%d:%02d:%02d" % [sec / 3600, (sec / 60) % 60, sec % 60]

func _slot_card(i: int, chapter: int, frames: int, enabled: bool) -> Button:
	var b := Button.new()
	b.custom_minimum_size = Vector2(900, 128)
	b.focus_mode = Control.FOCUS_ALL
	b.disabled = not enabled
	var n := Ui.stylebox("bar", 5)
	n.content_margin_left = 44
	n.content_margin_right = 44
	var h := Ui.stylebox("barsel", 5)
	h.content_margin_left = 44
	h.content_margin_right = 44
	var d := Ui.stylebox("bar", 5)
	d.content_margin_left = 44
	d.content_margin_right = 44
	d.modulate_color = Color(0.5, 0.5, 0.55, 0.8)
	b.add_theme_stylebox_override("normal", n)
	b.add_theme_stylebox_override("hover", h)
	b.add_theme_stylebox_override("focus", h)
	b.add_theme_stylebox_override("pressed", h)
	b.add_theme_stylebox_override("disabled", d)
	var row := HBoxContainer.new()
	row.set_anchors_preset(Control.PRESET_FULL_RECT)
	row.offset_left = 26
	row.offset_right = -26
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_theme_constant_override("separation", 22)
	b.add_child(row)
	var num := Label.new()
	num.text = str(i + 1)
	num.custom_minimum_size = Vector2(56, 0)
	num.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	num.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	num.add_theme_font_override("font", font_title)
	num.add_theme_font_size_override("font_size", 60)
	num.add_theme_color_override("font_color", gold if enabled else Color(0.5, 0.5, 0.5))
	num.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(num)
	var col := VBoxContainer.new()
	col.alignment = BoxContainer.ALIGNMENT_CENTER
	col.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	col.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(col)
	var t := Label.new()
	var name: String = core.chapter_title(chapter) if chapter >= 0 else T("Leerer Spielstand")
	var parts := name.split(":", true, 1)
	t.text = parts[1].strip_edges() if parts.size() > 1 else name
	t.add_theme_font_override("font", font_title)
	t.add_theme_font_size_override("font_size", 36)
	t.add_theme_color_override("font_color", Color(0.95, 0.96, 1.0) if chapter >= 0 else Color(0.6, 0.65, 0.7))
	t.mouse_filter = Control.MOUSE_FILTER_IGNORE
	col.add_child(t)
	var sub := Label.new()
	sub.text = (parts[0].strip_edges() + "   ·   " + T("Spielzeit") + " " + _fmt_time(frames)) if chapter >= 0 else T("Noch kein Spiel in diesem Platz")
	sub.add_theme_font_size_override("font_size", 22)
	sub.add_theme_color_override("font_color", Color(0.75, 0.85, 0.9, 0.85))
	sub.mouse_filter = Control.MOUSE_FILTER_IGNORE
	col.add_child(sub)
	b.pressed.connect(func(): _pick_slot(i))
	b.mouse_entered.connect(func(): if not b.disabled: b.grab_focus())
	return b

func _show_slots() -> void:
	for c in slot_cards.get_children():
		c.queue_free()
	var info: PackedInt32Array = core.slot_info()
	var first: Button = null
	var any_free := false
	for i in 3:
		if info.size() >= 6 and info[i * 2] < 0:
			any_free = true
	for i in 3:
		var ch := info[i * 2] if info.size() >= 6 else -1
		var fr := info[i * 2 + 1] if info.size() >= 6 else 0
		# new game: every slot (occupied ones are overwritten after a warning)
		var enabled := true if slot_bit == 4 or slot_bit == 9 else ch >= 0
		var b := _slot_card(i, ch, fr, enabled)
		slot_cards.add_child(b)
		# new game: start on the first empty slot so nothing gets overwritten by accident
		if enabled and first == null and (slot_bit != 4 or ch < 0):
			first = b
		if slot_bit == 4 and ch >= 0:
			b.tooltip_text = T("Achtung: Dieser Spielstand wird überschrieben.")
	var back := _button(T("Zurück"), func(): _slots_back())
	back.custom_minimum_size = Vector2(300, 60)
	var hb := HBoxContainer.new()
	hb.alignment = BoxContainer.ALIGNMENT_CENTER
	hb.add_child(back)
	slot_cards.add_child(hb)
	(first if first else back).call_deferred("grab_focus")
	slots_built = true

## The game asks where to save (end of a chapter): the native slot screen.
func _open_save_slots() -> void:
	if OS.has_environment("FE7HD_DEBUG"):
		print("open_save_slots f", Engine.get_process_frames())
	slot_bit = 9
	picking = false
	mode = Mode.SLOTS
	slots_built = false
	for c in slot_cards.get_children():
		c.queue_free()
	menu_root.visible = true
	menu_box.get_parent().get_parent().visible = false
	slot_panel.visible = true
	screen_rect.visible = false
	_open_slots_titles()

func _open_slots(bit: int) -> void:
	slot_bit = bit
	picking = false
	core.choose(bit)
	mode = Mode.SLOTS
	slots_built = false
	for c in slot_cards.get_children():
		c.queue_free()
	menu_box.get_parent().get_parent().visible = false
	slot_panel.visible = true
	_open_slots_titles()

func _open_slots_titles() -> void:
	slot_title.text = T(SLOT_TITLES.get(slot_bit, "Spielstand"))
	var sub := slot_panel.get_node("Sub") as Label
	sub.text = T({0: "Welchen Spielstand fortsetzen?", 1: "Welches Kapitel laden?", 3: "Welchen Spielstand löschen?", 4: "In welchem Platz soll das neue Spiel gespeichert werden?", 9: "In welchem Platz soll gespeichert werden?"}.get(slot_bit, ""))

func _pick_slot(i: int, confirmed := false) -> void:
	var info: PackedInt32Array = core.slot_info()
	var used := info.size() >= 6 and info[i * 2] >= 0
	if not confirmed and ((slot_bit == 4 and used) or (slot_bit == 9 and used) or slot_bit == 3):
		var qs := "Spielstand %d wirklich löschen?"
		if slot_bit == 4:
			qs = "Achtung: Spielstand %d wird gelöscht und überschrieben!\nTrotzdem ein neues Spiel beginnen?"
		elif slot_bit == 9:
			qs = "Spielstand %d überschreiben?"
		_ask(T(qs) % (i + 1),
			func(): _pick_slot(i, true), func(): _show_slots())
		return
	if slot_bit == 4 and used and any_free_slot(info):
		# the game itself only offers free slots while one is free: erase first, then start there
		core.slot_overwrite(i)
	else:
		core.slot_pick(i)
	slots_built = true
	picking = true
	for c in slot_cards.get_children():
		c.queue_free()
	(slot_panel.get_node("Sub") as Label).text = T("Einen Moment …")

func any_free_slot(info: PackedInt32Array) -> bool:
	for k in 3:
		if info.size() >= 6 and info[k * 2] < 0:
			return true
	return false

## Question with Ja/Nein inside the slot panel.
func _ask(question: String, yes: Callable, no: Callable) -> void:
	for c in slot_cards.get_children():
		c.queue_free()
	var q := Label.new()
	q.text = question
	q.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	q.add_theme_font_override("font", font_title)
	q.add_theme_font_size_override("font_size", 34)
	q.add_theme_color_override("font_color", Color(1, 0.92, 0.8))
	slot_cards.add_child(q)
	var hb := HBoxContainer.new()
	hb.alignment = BoxContainer.ALIGNMENT_CENTER
	hb.add_theme_constant_override("separation", 24)
	var by := _button(T("Ja"), yes)
	var bn := _button(T("Nein"), no)
	by.custom_minimum_size = Vector2(240, 64)
	bn.custom_minimum_size = Vector2(240, 64)
	hb.add_child(by)
	hb.add_child(bn)
	slot_cards.add_child(hb)
	bn.call_deferred("grab_focus")

func _slots_back() -> void:
	core.slot_back()
	if slot_bit == 9:
		slot_panel.visible = false
		_start_game(-1)
		return
	slot_panel.visible = false
	menu_box.get_parent().get_parent().visible = true
	_enter_menu()

func _process_slots() -> void:
	core.run(4 if core.busy() else 1, 0)
	_drain_audio()
	if picking and not core.busy():
		picking = false
		if core.in_slot_select() and slot_bit != 9:
			_show_slots()
			(slot_panel.get_node("Sub") as Label).text = T("Erledigt.")
		elif slot_bit == 3:
			# erased the last save: the game is back in its main menu
			slot_panel.visible = false
			menu_box.get_parent().get_parent().visible = true
			_enter_menu()
		else:
			slot_panel.visible = false
			menu_box.get_parent().get_parent().visible = true
			_start_game(-1)
		return
	if slot_bit == 9 and not picking and not core.in_slot_select():
		slot_panel.visible = false
		_start_game(-1)
		return
	if not slots_built and not core.busy():
		if core.in_slot_select():
			_show_slots()
		else:
			# the game went elsewhere (e.g. straight into a chapter)
			slot_panel.visible = false
			menu_box.get_parent().get_parent().visible = true
			_start_game(-1)

# ---------------------------------------------------------------- personal information (new game)
# Looks like the original screen (green marble, torn scroll, "name / birthmonth / gender"),
# but works like a modern form: type the name, click a value to pick it.

const MONTHS_SHORT := ["Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez"]
const GEMS := [Color(0.62, 0.08, 0.12), Color(0.55, 0.3, 0.75), Color(0.45, 0.8, 0.9), Color(0.93, 0.95, 1.0),
	Color(0.1, 0.7, 0.35), Color(0.95, 0.88, 0.9), Color(0.85, 0.05, 0.2), Color(0.6, 0.85, 0.2),
	Color(0.12, 0.3, 0.85), Color(0.9, 0.75, 0.85), Color(0.95, 0.7, 0.15), Color(0.25, 0.8, 0.75)]
const INK := Color(0.24, 0.12, 0.2)
const LABEL_INK := Color(0.55, 0.38, 0.22)

var f_uncial: FontFile
var f_narrow: FontFile
var ui_theme: Theme
var f_smooth: FontFile     # modern fallback for characters the game font lacks
var f_ui := FontFile.new()   # menu font for Godot labels (same glyphs, larger at the same size)
var f_px := FontFile.new()   # the game's menu font (from the ROM)
var f_talk := FontFile.new() # the game's dialogue font (from the ROM)
var p_stage: Control
var p_month_btn: Button
var p_gender_btn: Button
var p_bar: ColorRect
var p_emblem: Control
var p_popup: PanelContainer
var p_box_label: Label
var p_box_buttons: HBoxContainer
var p_month_idx := 0
var p_gender_idx := 0

## Builds a font from the game's own glyphs (16x16 cells, enlarged 4x so the
## pixels stay crisp at any size). Size 12 = the original size at 160 lines.
func _fill_font(f: FontFile, which: int, base := 48) -> void:
	var d: Dictionary = core.font_atlas(which)
	if not d.has("image"):
		return
	var img: Image = d.image
	img.resize(1024, 1024, Image.INTERPOLATE_NEAREST)
	f.antialiasing = TextServer.FONT_ANTIALIASING_NONE
	f.fixed_size = base
	f.fixed_size_scale_mode = TextServer.FIXED_SIZE_SCALE_ENABLED
	var sz := Vector2i(base, 0)
	f.set_texture_image(0, sz, 0, img)
	f.set_cache_ascent(0, base, 52)
	f.set_cache_descent(0, base, 12)
	var adv: PackedByteArray = d.adv
	for c in range(32, 256):
		if adv[c] == 0:
			continue
		f.set_glyph_advance(0, base, c, Vector2(adv[c] * 4, 0))
		f.set_glyph_offset(0, sz, c, Vector2(0, -52))
		f.set_glyph_size(0, sz, c, Vector2(64, 64))
		f.set_glyph_uv_rect(0, sz, c, Rect2((c % 16) * 64, (c / 16) * 64, 64, 64))
		f.set_glyph_texture_idx(0, sz, c, 0)
	if f_smooth:
		f.fallbacks = [f_smooth]

## A screen title as the game paints it (sprites in the given 240x160 area),
## cropped to the lettering. ink: bright lettering on a dark plate -> dark ink.
func _painted_title(off: bool, area: Rect2i, mode: int, ink: bool) -> Texture2D:
	if off:
		return null
	var img: Image = core.obj_region(area.position.x, area.position.y, area.size.x, area.size.y, mode)
	if img == null or img.is_invisible():
		return null
	if ink:
		for y in img.get_height():
			for x in img.get_width():
				var c := img.get_pixel(x, y)
				img.set_pixel(x, y, Color(0.28, 0.18, 0.08, c.a * clampf((c.v - 0.25) * 1.6, 0.0, 1.0)))
	var used := img.get_used_rect()
	if used.size.x < 8 or used.size.y < 6:
		return null
	return ImageTexture.create_from_image(img.get_region(used))

func _font(path: String) -> FontFile:
	var f := FontFile.new()
	f.load_dynamic_font(ProjectSettings.globalize_path(path))
	f.multichannel_signed_distance_field = true
	return f

func _lbl(text: String, font: Font, size: int, col: Color, pos: Vector2, parent: Control) -> Label:
	var l := Label.new()
	l.text = text
	l.add_theme_font_override("font", font)
	l.add_theme_font_size_override("font_size", size)
	l.add_theme_color_override("font_color", col)
	l.position = pos
	parent.add_child(l)
	return l

## Blue window like the game's menus (gradient, light rim, gold corners).
func _blue_window() -> StyleBox:
	var sb := Ui.stylebox("win", 4)
	sb.axis_stretch_vertical = StyleBoxTexture.AXIS_STRETCH_MODE_TILE
	sb.content_margin_left = 40
	sb.content_margin_right = 40
	sb.content_margin_top = 30
	sb.content_margin_bottom = 32
	return sb

func _flat(c: Color) -> StyleBoxFlat:
	var s := StyleBoxFlat.new()
	s.bg_color = c
	s.set_corner_radius_all(4)
	return s

func _value_button(text: String, pos: Vector2, w: float, cb: Callable) -> Button:
	var b := Button.new()
	b.text = text
	b.flat = true
	b.position = pos
	b.size = Vector2(w, 74)
	b.alignment = HORIZONTAL_ALIGNMENT_LEFT
	b.add_theme_font_override("font", f_talk)
	b.add_theme_font_size_override("font_size", 60)
	for k in ["font_color", "font_hover_color", "font_focus_color", "font_pressed_color"]:
		b.add_theme_color_override(k, INK)
	var empty := StyleBoxEmpty.new()
	for k in ["normal", "hover", "focus", "pressed"]:
		b.add_theme_stylebox_override(k, empty)
	b.pressed.connect(cb)
	b.mouse_entered.connect(func(): b.grab_focus())
	b.focus_entered.connect(func(): _bar_to(b))
	p_stage.add_child(b)
	return b

func _bar_to(c: Control) -> void:
	# lavender highlight bar behind the selected entry, as in the original
	p_bar.visible = true
	var tw := create_tween()
	tw.tween_property(p_bar, "position", c.position + Vector2(-14, 10), 0.08)
	tw.parallel().tween_property(p_bar, "size", Vector2(c.size.x + 28, c.size.y - 14), 0.08)

func _build_personal() -> void:
	f_uncial = _font("res://fonts/fell.woff2")
	f_smooth = _font("res://fonts/narrow-bold.woff2")
	f_narrow = f_px
	personal_root = Control.new()
	personal_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	personal_root.visible = false
	ui_layer.add_child(personal_root)
	var marble := ColorRect.new()
	marble.set_anchors_preset(Control.PRESET_FULL_RECT)
	var mm := ShaderMaterial.new()
	mm.shader = load("res://shaders/marble.gdshader")
	marble.material = mm
	personal_root.add_child(marble)
	# fixed 1600x900 stage, centred
	var center := CenterContainer.new()
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	personal_root.add_child(center)
	p_stage = Control.new()
	p_stage.custom_minimum_size = Vector2(1600, 900)
	center.add_child(p_stage)
	var parch := ColorRect.new()
	parch.position = Vector2(200, 40)
	parch.size = Vector2(1200, 760)
	var pm := ShaderMaterial.new()
	pm.shader = load("res://shaders/parchment.gdshader")
	pm.set_shader_parameter("size", parch.size)
	parch.material = pm
	parch.mouse_filter = Control.MOUSE_FILTER_IGNORE
	p_stage.add_child(parch)
	p_bar = ColorRect.new()
	p_bar.color = Color(0.52, 0.55, 0.95, 0.45)
	p_bar.visible = false
	p_bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
	p_stage.add_child(p_bar)
	var title := _lbl(T("Persönliche Daten"), f_talk, 74, Color(0.2, 0.1, 0.05), Vector2(380, 84), p_stage)
	_tr_nodes.append([title, "Persönliche Daten"])
	title.add_theme_color_override("font_shadow_color", Color(1, 0.86, 0.6, 0.7))
	title.add_theme_constant_override("shadow_offset_x", 2)
	title.add_theme_constant_override("shadow_offset_y", 2)
	for spec in [["name", Vector2(560, 210)], ["geburtsmonat", Vector2(380, 380)], ["geschlecht", Vector2(930, 380)]]:
		var l := _lbl(T(spec[0]), f_talk, 46, LABEL_INK, spec[1] - Vector2(0, 6), p_stage)
		_tr_nodes.append([l, spec[0]])
		l.add_theme_color_override("font_shadow_color", Color(0.3, 0.16, 0.06, 0.6))
		l.add_theme_constant_override("shadow_offset_x", 2)
		l.add_theme_constant_override("shadow_offset_y", 2)
	# birthstone emblem next to the name
	p_emblem = Control.new()
	p_emblem.position = Vector2(720, 262)
	p_emblem.size = Vector2(56, 56)
	p_emblem.mouse_filter = Control.MOUSE_FILTER_IGNORE
	p_emblem.draw.connect(func():
		var c := Vector2(28, 28)
		p_emblem.draw_circle(c, 27, Color(0.35, 0.2, 0.45))
		p_emblem.draw_circle(c, 23, Color(0.93, 0.78, 0.4))
		var g: Color = GEMS[p_month_idx]
		var pts := PackedVector2Array([c + Vector2(0, -17), c + Vector2(17, 0), c + Vector2(0, 17), c + Vector2(-17, 0)])
		p_emblem.draw_colored_polygon(pts, g)
		p_emblem.draw_colored_polygon(PackedVector2Array([c + Vector2(0, -17), c + Vector2(17, 0), c]), g.lightened(0.45))
		p_emblem.draw_polyline(PackedVector2Array([pts[0], pts[1], pts[2], pts[3], pts[0]]), Color(0.25, 0.12, 0.05), 2.0))
	p_stage.add_child(p_emblem)
	p_name = LineEdit.new()
	p_name.position = Vector2(790, 250)
	p_name.size = Vector2(380, 78)
	p_name.max_length = 7
	p_name.add_theme_font_override("font", f_talk)
	p_name.add_theme_font_size_override("font_size", 60)
	p_name.add_theme_color_override("font_color", INK)
	p_name.add_theme_color_override("caret_color", INK)
	p_name.add_theme_color_override("selection_color", Color(0.52, 0.55, 0.95, 0.6))
	p_name.add_theme_color_override("font_selected_color", INK)
	p_name.select_all_on_focus = false
	p_name.add_theme_color_override("font_placeholder_color", Color(0.4, 0.3, 0.3, 0.5))
	var empty := StyleBoxEmpty.new()
	for k in ["normal", "focus", "read_only"]:
		p_name.add_theme_stylebox_override(k, empty)
	p_name.focus_entered.connect(func(): _bar_to(p_name); _close_popup())
	p_name.text_submitted.connect(func(_t): p_month_btn.grab_focus())
	p_stage.add_child(p_name)
	p_month_btn = _value_button("Januar", Vector2(430, 430), 380, func(): _open_months())
	p_gender_btn = _value_button("Männlich", Vector2(980, 430), 300, func(): _open_genders())
	# yellow message box at the bottom (like the original's)
	var box := PanelContainer.new()
	var yb := StyleBoxFlat.new()
	yb.bg_color = Color(0.93, 0.93, 0.55)
	yb.border_color = Color(0.72, 0.62, 0.15)
	yb.set_border_width_all(4)
	yb.set_corner_radius_all(4)
	yb.content_margin_left = 28
	yb.content_margin_right = 28
	yb.content_margin_top = 14
	yb.content_margin_bottom = 14
	box.add_theme_stylebox_override("panel", yb)
	box.position = Vector2(420, 610)
	box.size = Vector2(780, 130)
	p_stage.add_child(box)
	var bv := VBoxContainer.new()
	box.add_child(bv)
	p_box_label = Label.new()
	p_box_label.add_theme_font_override("font", f_talk)
	p_box_label.add_theme_font_size_override("font_size", 36)
	p_box_label.add_theme_color_override("font_color", Color(0.15, 0.12, 0.1))
	bv.add_child(p_box_label)
	p_box_buttons = HBoxContainer.new()
	p_box_buttons.add_theme_constant_override("separation", 40)
	bv.add_child(p_box_buttons)
	p_ok = Button.new()  # kept for the test hook / state
	# help strip at the bottom of the screen
	var strip := ColorRect.new()
	strip.color = Color(0, 0, 0, 0.45)
	strip.position = Vector2(0, 846)
	strip.size = Vector2(1600, 54)
	p_stage.add_child(strip)
	var hint := _lbl("", f_narrow, 30, Color(1, 1, 1, 0.9), Vector2(40, 852), p_stage)
	_tt(hint, "Eintrag anklicken oder Pfeiltasten + Enter  ·  Namen einfach tippen  ·  Esc schließt Auswahl")
	p_popup = PanelContainer.new()
	p_popup.add_theme_stylebox_override("panel", _blue_window())
	p_popup.visible = false
	p_stage.add_child(p_popup)
	_box_idle()

func _box_word(text: String, cb: Callable) -> Button:
	var b := Button.new()
	b.text = text
	b.flat = true
	b.add_theme_font_override("font", f_talk)
	b.add_theme_font_size_override("font_size", 38)
	b.add_theme_color_override("font_color", Color(0.15, 0.12, 0.1))
	b.add_theme_color_override("font_hover_color", Color(0.1, 0.1, 0.5))
	b.add_theme_color_override("font_focus_color", Color(0.1, 0.1, 0.5))
	var hl := _flat(Color(0.52, 0.55, 0.95, 0.45))
	b.add_theme_stylebox_override("normal", StyleBoxEmpty.new())
	b.add_theme_stylebox_override("hover", hl)
	b.add_theme_stylebox_override("focus", hl)
	b.add_theme_stylebox_override("pressed", hl)
	b.pressed.connect(cb)
	b.mouse_entered.connect(func(): b.grab_focus())
	return b

func _box_idle() -> void:
	p_box_label.text = T("Lege Namen, Geburtsmonat und Geschlecht fest.")
	for c in p_box_buttons.get_children():
		c.queue_free()
	p_box_buttons.add_child(_box_word(T("▶ Fertig"), func(): _box_confirm()))

func _box_confirm() -> void:
	_close_popup()
	p_box_label.text = T("Sind das die gewünschten Angaben?")
	for c in p_box_buttons.get_children():
		c.queue_free()
	var yes := _box_word(T("Ja"), func(): _personal_done())
	p_box_buttons.add_child(yes)
	p_box_buttons.add_child(_box_word(T("Nein"), func(): _box_idle(); p_name.grab_focus()))
	yes.call_deferred("grab_focus")

func _close_popup() -> void:
	if p_popup and p_popup.visible:
		p_popup.visible = false

func _popup_word(text: String, cb: Callable) -> Button:
	var b := Button.new()
	b.text = text
	b.flat = true
	b.custom_minimum_size = Vector2(120, 58)
	b.add_theme_font_override("font", f_narrow)
	b.add_theme_font_size_override("font_size", 44)
	for k in ["font_color", "font_hover_color", "font_focus_color", "font_pressed_color"]:
		b.add_theme_color_override(k, Color(0.97, 0.97, 1.0))
	var hl := _flat(Color(1, 1, 1, 0.22))
	b.add_theme_stylebox_override("normal", StyleBoxEmpty.new())
	b.add_theme_stylebox_override("hover", hl)
	b.add_theme_stylebox_override("focus", hl)
	b.add_theme_stylebox_override("pressed", hl)
	b.pressed.connect(cb)
	b.mouse_entered.connect(func(): b.grab_focus())
	return b

func _open_months() -> void:
	for c in p_popup.get_children():
		c.queue_free()
	var g := GridContainer.new()
	g.columns = 6
	g.add_theme_constant_override("h_separation", 18)
	p_popup.add_child(g)
	var cur: Button = null
	for i in 12:
		var b := _popup_word(T(MONTHS_SHORT[i]), func(): _set_month(i); _close_popup(); p_month_btn.grab_focus())
		g.add_child(b)
		if i == p_month_idx:
			cur = b
	p_popup.position = Vector2(360, 520)
	p_popup.size = Vector2.ZERO
	p_popup.visible = true
	cur.call_deferred("grab_focus")

func _open_genders() -> void:
	for c in p_popup.get_children():
		c.queue_free()
	var h := HBoxContainer.new()
	h.add_theme_constant_override("separation", 24)
	p_popup.add_child(h)
	var bm := _popup_word(T("Männlich"), func(): _set_gender(0); _close_popup(); p_gender_btn.grab_focus())
	var bw := _popup_word(T("Weiblich"), func(): _set_gender(1); _close_popup(); p_gender_btn.grab_focus())
	h.add_child(bm)
	h.add_child(bw)
	p_popup.position = Vector2(930, 520)
	p_popup.size = Vector2.ZERO
	p_popup.visible = true
	(bw if p_gender_idx == 1 else bm).call_deferred("grab_focus")

func _set_month(i: int) -> void:
	p_month_idx = i
	p_month_btn.text = T(MONTHS[i])
	p_emblem.queue_redraw()

func _set_gender(i: int) -> void:
	p_gender_idx = i
	p_gender_btn.text = T("Weiblich" if i == 1 else "Männlich")

func _personal_show() -> void:
	personal_state = 1
	var cur: Array = core.personal_get()
	p_name.text = str(cur[0]) if str(cur[0]) != "" else "Mark"
	_set_month(int(cur[1]))
	_set_gender(int(cur[2]))
	_box_idle()
	personal_root.visible = true
	screen_rect.visible = false
	p_name.call_deferred("grab_focus")
	p_name.caret_column = p_name.text.length()

func _personal_bg() -> void:
	pass

func _personal_done() -> void:
	var n := p_name.text.strip_edges()
	if n == "":
		n = "Mark"
	core.personal_set(n, p_month_idx, p_gender_idx)
	personal_state = 2
	p_box_label.text = T("Einen Moment …")
	for c in p_box_buttons.get_children():
		c.queue_free()

func _personal_update() -> bool:
	# returns true while the native screen replaces the game picture
	if personal_state == 0:
		if core.personal_active() and not core.busy():
			_personal_show()
			return true
		return false
	if personal_state == 1 and core.personal_active():
		return true
	if personal_state == 2 and core.busy():
		return true
	personal_state = 0
	personal_root.visible = false
	screen_rect.visible = true
	return false

## Native unit status screen (over the whole window, green marble like the original).
func _build_status() -> void:
	status_root = Control.new()
	status_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	status_root.visible = false
	status_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	ui_layer.add_child(status_root)
	minimap_ui = preload("res://scripts/minimap.gd").new()
	minimap_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	minimap_ui.font = f_narrow
	minimap_ui.title_font = f_talk
	minimap_ui.visible = false
	ui_layer.add_child(minimap_ui)
	prepbar_ui = preload("res://scripts/prepbar.gd").new()
	prepbar_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	prepbar_ui.font = f_narrow
	prepbar_ui.title_font = f_talk
	prepbar_ui.visible = false
	prepbar_ui.key.connect(func(k: int):
		key_queue.append([k, 4])
		key_queue.append([0, 3]))
	ui_layer.add_child(prepbar_ui)
	var marble := ColorRect.new()
	marble.set_anchors_preset(Control.PRESET_FULL_RECT)
	var mm := ShaderMaterial.new()
	mm.shader = load("res://shaders/marble.gdshader")
	marble.material = mm
	marble.mouse_filter = Control.MOUSE_FILTER_IGNORE
	status_root.add_child(marble)
	# the game's own background picture of the open screen (its lowest layer)
	status_bg = TextureRect.new()
	status_bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	status_bg.stretch_mode = TextureRect.STRETCH_SCALE
	status_bg.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	status_bg.mouse_filter = Control.MOUSE_FILTER_IGNORE
	status_root.add_child(status_bg)
	status_ui = preload("res://scripts/status.gd").new()
	status_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	status_ui.font = f_narrow
	status_ui.title_font = f_talk
	status_ui.ink_font = f_talk
	status_ui.key.connect(func(k: int):
		if k != 0:
			key_queue.append([k, 4])
			key_queue.append([0, 2]))
	status_root.add_child(status_ui)
	unitlist_ui = preload("res://scripts/unitlist.gd").new()
	unitlist_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	unitlist_ui.font = f_narrow
	unitlist_ui.title_font = f_talk
	unitlist_ui.ink_font = f_talk
	unitlist_ui.visible = false
	unitlist_ui.key.connect(func(k: int):
		key_queue.append([k, 4])
		key_queue.append([0, 3]))
	status_root.add_child(unitlist_ui)
	options_ui = preload("res://scripts/options.gd").new()
	options_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	options_ui.font = f_narrow
	options_ui.title_font = f_talk
	options_ui.ink_font = f_talk
	options_ui.visible = false
	options_ui.key.connect(func(k: int):
		key_queue.append([k, 4])
		key_queue.append([0, 3]))
	status_root.add_child(options_ui)
	prep_ui = preload("res://scripts/prep.gd").new()
	prep_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	prep_ui.font = f_narrow
	prep_ui.title_font = f_talk
	prep_ui.ink_font = f_talk
	prep_ui.visible = false
	prep_ui.key.connect(func(k: int):
		key_queue.append([k, 4])
		key_queue.append([0, 3]))
	status_root.add_child(prep_ui)
	prepunits_ui = preload("res://scripts/prepunits.gd").new()
	prepunits_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	prepunits_ui.font = f_narrow
	prepunits_ui.title_font = f_talk
	prepunits_ui.ink_font = f_talk
	prepunits_ui.visible = false
	prepunits_ui.key.connect(func(k: int):
		key_queue.append([k, 4])
		key_queue.append([0, 3]))
	status_root.add_child(prepunits_ui)

## Shows the native status screen while the game is in it; true while shown.
func _status_update() -> bool:
	var sd: Dictionary = core.status() if settings.menus else {}
	var ud: Dictionary = {} if not sd.is_empty() or not settings.menus else core.unitlist()
	var od: Dictionary = {} if not settings.menus else core.options()
	var pd: Dictionary = {} if not settings.menus else core.prep()
	prep_ui.labels = {"Preparations": T("Vorbereitung"), "Goal": T("Ziel"), "Funds": T("Kriegskasse"), "Fight!": T("Zum Kampf!"), "Command Help": T("Befehlshilfe")}
	prep_ui.title_tex = _painted_title(pd.is_empty(), Rect2i(0, 0, 100, 22), -1, true)
	options_ui.title_tex = _painted_title(od.is_empty(), Rect2i(16, 2, 112, 22), -1, false)
	prep_ui.show_prep(pd)
	options_ui.show_options(od)
	var qd: Dictionary = {} if not settings.menus or not sd.is_empty() or not ud.is_empty() else core.prepunits()
	if not prepunits_ui.icon_cb.is_valid():
		prepunits_ui.icon_cb = trade_ui.icon_cb
	prepunits_ui.labels = {"Pick Units": T("Truppe wählen"), "Trade": T("Tauschen"), "Goal": T("Ziel"),
		"more": T("Noch %d Einheiten wählen"), "ready": T("Truppe vollständig"), "Choose unit": T("Einheit wählen"),
		"Trade with": T("%s tauscht mit …"), "Back": T("Zurück"), "Info": T("Info"), "Unit List": T("Einheitenliste"),
		"Fight!": T("Zum Kampf!")}
	prepunits_ui.show_units(qd)
	if not od.is_empty() or not pd.is_empty() or not qd.is_empty():
		status_ui.visible = false
		unitlist_ui.visible = false
		if not status_root.visible:
			status_root.visible = true
			screen_rect.visible = false
		return true
	if sd.is_empty() and ud.is_empty():
		if status_root.visible:
			status_root.visible = false
			screen_rect.visible = true
		return false
	if not status_root.visible:
		status_root.visible = true
		screen_rect.visible = false
	status_ui.visible = not sd.is_empty()
	if not sd.is_empty():
		unitlist_ui.visible = false
		status_ui.title_tex = _painted_title(false, Rect2i(116, 2, 86, 20), -3, false)
		status_ui.show_status(sd, core.frame())
		return true
	var icons := {}
	for ic in ud.icons:
		var row := (int(ic[2]) - 56) / 16
		var img: Image = core.obj_image(ic[0], ic[1], 16, 16)
		if img and not img.is_invisible():
			icons[row] = ImageTexture.create_from_image(img)
	unitlist_ui.title_tex = _painted_title(ud.is_empty(), Rect2i(8, 4, 112, 18), -1, false)
	unitlist_ui.show_list(ud, icons)
	return true

func _rebuild_menu_items(mask: int) -> void:
	for c in menu_box.get_children():
		c.queue_free()
	var first: Button = null
	var items := []
	if core and not (chapter_panel and chapter_panel.visible):
		core.debug_chapter(-1)   # back in the main menu: a chosen debug chapter was cancelled
	if mask & 1:
		items.append(["Fortsetzen", func(): _open_slots(0)])
	if mask & 2:
		items.append(["Kapitel laden", func(): _open_slots(1)])
	if mask & 16:
		items.append(["Neues Spiel", func(): _open_slots(4)])
	if mask & 8:
		items.append(["Spielstand löschen", func(): _open_slots(3)])
	if settings.debug:
		items.append(["Kapitel wählen (Debug)", func(): _open_chapters()])
	items.append(["Einstellungen", func(): _open_settings()])
	items.append(["Beenden", func(): _quit()])
	for it in items:
		var b := _button(T(it[0]), it[1])
		menu_box.add_child(b)
		if first == null:
			first = b
	if first:
		first.call_deferred("grab_focus")

func _opt_row(parent: Control, de_label: String, options: Array, cur: int, cb: Callable) -> OptionButton:
	var hl := HBoxContainer.new()
	var l := Label.new()
	_tt(l, de_label)
	l.add_theme_font_size_override("font_size", 24)
	l.custom_minimum_size.x = 250
	hl.add_child(l)
	var ob := OptionButton.new()
	ob.add_theme_font_size_override("font_size", 22)
	ob.custom_minimum_size = Vector2(240, 40)
	for o in options:
		ob.add_item(T(o) if o is String else str(o))
	ob.select(cur)
	ob.item_selected.connect(cb)
	ob.mouse_entered.connect(func(): ob.grab_focus())
	hl.add_child(ob)
	parent.add_child(hl)
	return ob

func _build_settings() -> void:
	settings_panel = PanelContainer.new()
	settings_panel.add_theme_stylebox_override("panel", _panel_style(0.94))
	settings_panel.set_anchors_preset(Control.PRESET_CENTER)
	settings_panel.grow_horizontal = Control.GROW_DIRECTION_BOTH
	settings_panel.grow_vertical = Control.GROW_DIRECTION_BOTH
	settings_panel.visible = false
	ui_layer.add_child(settings_panel)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 12)
	settings_panel.add_child(v)
	var t := Label.new()
	_tt(t, "Einstellungen")
	t.add_theme_font_override("font", font_title)
	t.add_theme_font_size_override("font_size", 40)
	t.add_theme_color_override("font_color", gold)
	t.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	v.add_child(t)
	var cols := HBoxContainer.new()
	cols.add_theme_constant_override("separation", 40)
	v.add_child(cols)
	var left := VBoxContainer.new()
	left.add_theme_constant_override("separation", 8)
	cols.add_child(left)
	var right := VBoxContainer.new()
	right.add_theme_constant_override("separation", 12)
	cols.add_child(right)
	for spec in [["fullscreen", "Vollbild"], ["hd", "HD-2D-Karte (aus = Originalgrafik)"], ["dof", "Tiefenunschärfe"], ["shadows", "Schatten"],
			["talk", "Sprechblasen neu gezeichnet"], ["menus", "Menüs neu gezeichnet"], ["instant", "Text sofort ganz zeigen"], ["fps", "FPS anzeigen"],
			["debug", "Debug-Modus (Kapitel wählen)"]]:
		var cb := CheckButton.new()
		_tt(cb, spec[1])
		cb.add_theme_font_size_override("font_size", 24)
		cb.button_pressed = settings[spec[0]]
		var key: String = spec[0]
		cb.toggled.connect(func(on): settings[key] = on; _apply_all())
		cb.mouse_entered.connect(func(): cb.grab_focus())
		left.add_child(cb)
	lang_opt = _opt_row(right, "Sprache", ["English", "Deutsch"], 1 if settings.lang == "de" else 0,
		func(i): settings.lang = "de" if i == 1 else "en"; _apply_all(); _retranslate())
	var q := _opt_row(right, "Grafik", ["Niedrig", "Mittel", "Hoch"], int(settings.quality),
		func(i): settings.quality = i; _apply_all())
	_tr_opts.append([q, ["Niedrig", "Mittel", "Hoch"]])
	var speeds := [2, 4, 8]
	_opt_row(right, "Vorspulen (Strg)", ["2×", "4×", "8×"], maxi(0, speeds.find(int(settings.ffwd))),
		func(i): settings.ffwd = speeds[i]; _apply_all())
	var hl := HBoxContainer.new()
	var l := Label.new()
	_tt(l, "Lautstärke")
	l.add_theme_font_size_override("font_size", 24)
	l.custom_minimum_size.x = 250
	hl.add_child(l)
	var s := HSlider.new()
	s.min_value = 0
	s.max_value = 1
	s.step = 0.05
	s.value = settings.volume
	s.custom_minimum_size = Vector2(240, 36)
	s.value_changed.connect(func(x): settings.volume = x; _apply_all())
	hl.add_child(s)
	right.add_child(hl)
	var keys := Label.new()
	_tt(keys, "STEUERUNG_HILFE")
	keys.add_theme_font_size_override("font_size", 26)
	keys.add_theme_color_override("font_color", Color(0.8, 0.85, 1, 0.8))
	v.add_child(keys)
	var back := _button(T("Zurück"), func(): _close_settings())
	_tr_nodes.append([back, "Zurück"])
	var bh := HBoxContainer.new()
	bh.alignment = BoxContainer.ALIGNMENT_CENTER
	bh.add_child(back)
	v.add_child(bh)

var _tr_opts: Array = []  # [OptionButton, german items]
var lang_opt: OptionButton

func _build_pause() -> void:
	pause_panel = PanelContainer.new()
	pause_panel.add_theme_stylebox_override("panel", _panel_style(0.92))
	pause_panel.set_anchors_preset(Control.PRESET_CENTER)
	pause_panel.grow_horizontal = Control.GROW_DIRECTION_BOTH
	pause_panel.grow_vertical = Control.GROW_DIRECTION_BOTH
	pause_panel.visible = false
	ui_layer.add_child(pause_panel)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 12)
	pause_panel.add_child(v)
	var t := Label.new()
	_tt(t, "Pause")
	t.add_theme_font_override("font", font_title)
	t.add_theme_font_size_override("font_size", 40)
	t.add_theme_color_override("font_color", gold)
	t.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	v.add_child(t)
	for spec in [["Weiterspielen", func(): _set_paused(false)], ["Schnellspeichern (F5)", func(): _quick_save()],
			["Schnellladen (F8)", func(): _quick_load()], ["Einstellungen", func(): _open_settings()],
			["Kapitel wählen (Debug)", func(): _to_menu(); _open_chapters()],
			["Hauptmenü", func(): _to_menu()], ["Beenden", func(): _quit()]]:
		var b := _button(T(spec[0]), spec[1])
		_tr_nodes.append([b, spec[0]])
		v.add_child(b)
		if spec[0] == "Kapitel wählen (Debug)":
			debug_pause_btn = b
	# FPS counter and short notices
	fps_label = Label.new()
	fps_label.add_theme_font_size_override("font_size", 18)
	fps_label.add_theme_color_override("font_color", Color(1, 1, 0.7))
	fps_label.add_theme_color_override("font_outline_color", Color(0, 0, 0))
	fps_label.add_theme_constant_override("outline_size", 4)
	fps_label.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	fps_label.position = Vector2(-120, 8)
	fps_label.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	fps_label.visible = settings.fps
	ui_layer.add_child(fps_label)
	toast = Label.new()
	toast.add_theme_font_override("font", font_title)
	toast.add_theme_font_size_override("font_size", 28)
	toast.add_theme_color_override("font_color", Color(1, 0.95, 0.8))
	toast.add_theme_color_override("font_outline_color", Color(0.1, 0.06, 0.02))
	toast.add_theme_constant_override("outline_size", 8)
	toast.set_anchors_preset(Control.PRESET_CENTER_TOP)
	toast.grow_horizontal = Control.GROW_DIRECTION_BOTH
	toast.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	toast.position.y = 24
	toast.modulate.a = 0.0
	toast.mouse_filter = Control.MOUSE_FILTER_IGNORE
	ui_layer.add_child(toast)

## Debug: list of all chapters; the next chapter start loads the one picked.
var chapter_panel: PanelContainer
var chapter_start: OptionButton
var debug_pause_btn: Button
const CHAPTER_GROUPS := [["Lyns Geschichte", 0x00, 0x0B], ["Eliwoods Geschichte", 0x0C, 0x28], ["Hectors Geschichte", 0x29, 0x45]]

func _open_chapters() -> void:
	if chapter_panel == null:
		_build_chapters()
	menu_root.visible = true
	menu_box.get_parent().get_parent().visible = false
	chapter_start.set_item_disabled(0, not (menu_built_mask & 2))
	if not (menu_built_mask & 2) and menu_built_mask >= 0:
		chapter_start.select(1)
	chapter_panel.visible = true
	var k := minf(1.0, minf(get_viewport().get_visible_rect().size.x * 0.96 / chapter_panel.size.x,
		get_viewport().get_visible_rect().size.y * 0.96 / chapter_panel.size.y))
	chapter_panel.pivot_offset = chapter_panel.size * 0.5
	chapter_panel.scale = Vector2(k, k)
	chapter_start.call_deferred("grab_focus")

func _close_chapters() -> void:
	chapter_panel.visible = false
	if mode == Mode.MENU:
		menu_box.get_parent().get_parent().visible = true
		_rebuild_menu_items(menu_built_mask)

func _build_chapters() -> void:
	chapter_panel = PanelContainer.new()
	chapter_panel.add_theme_stylebox_override("panel", _panel_style(0.95))
	chapter_panel.set_anchors_preset(Control.PRESET_CENTER)
	chapter_panel.grow_horizontal = Control.GROW_DIRECTION_BOTH
	chapter_panel.grow_vertical = Control.GROW_DIRECTION_BOTH
	chapter_panel.visible = false
	ui_layer.add_child(chapter_panel)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 10)
	chapter_panel.add_child(v)
	var t := Label.new()
	_tt(t, "Kapitel wählen")
	t.add_theme_font_override("font", font_title)
	t.add_theme_font_size_override("font_size", 40)
	t.add_theme_color_override("font_color", Color(0.98, 0.9, 0.62))
	t.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	v.add_child(t)
	chapter_start = _opt_row(v, "Start mit", ["Einheiten aus einem Spielstand", "Neues Spiel (nur Einheiten des Kapitels)"], 0, func(_i): pass)
	chapter_start.custom_minimum_size.x = 560
	_tr_opts.append([chapter_start, ["Einheiten aus einem Spielstand", "Neues Spiel (nur Einheiten des Kapitels)"]])
	var note := Label.new()
	_tt(note, "Der Spielstand wird zu Kapitelbeginn überschrieben. Der Story-Modus (Lyn/Eliwood/Hector) bleibt der des Spielstands.")
	note.add_theme_font_size_override("font_size", 20)
	note.add_theme_color_override("font_color", Color(0.8, 0.85, 1, 0.8))
	v.add_child(note)
	var sc := ScrollContainer.new()
	sc.custom_minimum_size = Vector2(1400, 560)
	sc.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	v.add_child(sc)
	var list := VBoxContainer.new()
	list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	list.add_theme_constant_override("separation", 8)
	sc.add_child(list)
	for grp in CHAPTER_GROUPS:
		var h := Label.new()
		_tt(h, grp[0])
		h.add_theme_font_override("font", font_title)
		h.add_theme_font_size_override("font_size", 30)
		h.add_theme_color_override("font_color", Color(0.98, 0.9, 0.62))
		list.add_child(h)
		var grid := GridContainer.new()
		grid.columns = 3
		grid.add_theme_constant_override("h_separation", 8)
		grid.add_theme_constant_override("v_separation", 6)
		list.add_child(grid)
		var seen := {}
		for id in range(grp[1], grp[2] + 1):
			var name: String = core.chapter_title(id)
			seen[name] = int(seen.get(name, 0)) + 1
			if seen[name] > 1:
				name += "  (%d)" % seen[name]   # route variants share a title
			var b := _button(name, _pick_chapter.bind(id))
			b.custom_minimum_size = Vector2(450, 46)
			b.add_theme_font_size_override("font_size", 20)
			b.alignment = HORIZONTAL_ALIGNMENT_LEFT
			b.clip_text = true
			b.tooltip_text = "0x%02X" % id
			grid.add_child(b)
	var back := _button(T("Zurück"), func(): _close_chapters())
	_tr_nodes.append([back, "Zurück"])
	back.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	v.add_child(back)

func _pick_chapter(id: int) -> void:
	chapter_panel.visible = false
	core.debug_chapter(id)
	var bit := 1 if chapter_start.selected == 0 and (menu_built_mask & 2) else 4
	_open_slots(bit)
	_toast("Debug: " + core.chapter_title(id))

## Test: loads every chapter from first to last (debug chapter + restart from slot 1),
## skips its scenes and saves an overview of the 3D map per chapter, then quits.
var touring := false

func _chapter_tour(first: int, last: int) -> void:
	touring = true
	settings.dof = false
	_apply_all()
	DirAccess.make_dir_recursive_absolute(shot_dir())
	for id in range(first, last + 1):
		core.reset_to_menu()
		for k in 400:
			if not core.busy() and core.menu_mask() != 0:
				break
			core.run(20, 0)
		core.debug_chapter(id)
		core.choose(1)
		for k in 400:
			if core.in_slot_select():
				break
			core.run(5, 0)
		core.slot_pick(0)
		var f := 0
		while f < 6000:
			var keys := 0
			var ph := f % 48
			if ph < 4:
				keys = 8      # START: skip scenes
			elif ph >= 24 and ph < 28:
				keys = 2      # B: close what START opened
			core.run(1, keys)
			f += 1
			if f > 1500 and core.is_map() and core.chapter() == id and core.debug_chapter_pending() < 0 and f % 480 == 0:
				break
		for k in 120:
			core.run(1, 2 if k % 20 < 3 else 0)
		mode = Mode.GAME
		menu_root.visible = false
		screen_rect.visible = true
		for n in find_children("*", "CanvasLayer", true, false):
			(n as CanvasLayer).visible = false
		world.cam_ready = false
		for k in 4:
			core.run(1, 0)
			_present(1.0 / 60.0)
			await get_tree().process_frame
		var ms: Vector2i = core.map_size()
		var vv: Vector2i = core.view()
		world.yaw = 0.0
		world.pitch_deg = 52.0
		world.zoom = maxf(ms.x, ms.y * 1.6) / 11.0
		world.pan = Vector3(ms.x * 0.5 - (vv.x + 120) / 16.0, 0, ms.y * 0.5 - (vv.y + 80) / 16.0)
		var tc := OS.get_environment("FE7HD_TOUR_CAM").split(",")
		if tc.size() == 5:
			world.yaw = float(tc[0])
			world.pitch_deg = float(tc[1])
			world.zoom = float(tc[2])
			world.pan = Vector3(float(tc[3]) + 0.5 - (vv.x + 120) / 16.0, 0, float(tc[4]) + 0.5 - (vv.y + 80) / 16.0)
		world._yaw_s = world.yaw
		world._pitch_s = world.pitch_deg
		world._zoom_s = world.zoom
		world.cam_ready = false
		for k in 6:
			_present(1.0 / 60.0)
			world._update_camera(1.0)
			await get_tree().process_frame
		if not OS.get_environment("FE7HD_TOUR_TYPES").is_empty():
			await _tour_types(id, ms, vv)
		else:
			get_viewport().get_texture().get_image().save_png(shot_dir() + "/tour_%02x.png" % id)
		printerr("TOUR %02x ch%d map %s %s" % [id, core.chapter(), str(ms), core.chapter_title(id)])
	get_tree().quit()

var tour_seen := {}
const TOUR_SKIP := [0x00, 0x01, 0x02, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x15, 0x16, 0x17, 0x18]

## Test: close-up of the first two cells of every terrain type met on the tour.
func _tour_types(id: int, ms: Vector2i, vv: Vector2i) -> void:
	var terr: PackedByteArray = core.terrain(0)
	var hist := {}
	for v in terr:
		hist[v & 0x7F] = int(hist.get(v & 0x7F, 0)) + 1
	printerr("HIST %02x %s" % [id, str(hist)])
	for i in terr.size():
		var t: int = terr[i] & 0x7F
		if t in TOUR_SKIP or int(tour_seen.get(t, 0)) >= 2:
			continue
		if tour_seen.has(t) and tour_seen.get("last%d" % t, -1) == id:
			continue
		tour_seen[t] = int(tour_seen.get(t, 0)) + 1
		tour_seen["last%d" % t] = id
		var cx := i % ms.x
		var cy := i / ms.x
		world.yaw = 20.0
		world.pitch_deg = 38.0
		world.zoom = 0.42
		world.pan = Vector3(cx + 0.5 - (vv.x + 120) / 16.0, 0, cy + 0.5 - (vv.y + 80) / 16.0)
		world._yaw_s = world.yaw
		world._pitch_s = world.pitch_deg
		world._zoom_s = world.zoom
		world.cam_ready = false
		for k in 4:
			world._update_camera(1.0)
			await get_tree().process_frame
		get_viewport().get_texture().get_image().save_png(shot_dir() + "/type_%02x_%d_ch%02x_%d_%d.png" % [t, tour_seen[t], id, cx, cy])

func _toast(de: String) -> void:
	toast.text = T(de)
	toast_t = 2.0

func _can_quick() -> bool:
	return mode == Mode.GAME and personal_state == 0 and not core.busy()

func _quick_save() -> void:
	if not _can_quick():
		return
	if core.save_state(ProjectSettings.globalize_path(QUICK)):
		_toast("Schnellgespeichert")
	_set_paused(false)

func _quick_load() -> void:
	if mode != Mode.GAME or personal_state != 0:
		return
	if not FileAccess.file_exists(QUICK):
		_toast("Kein Schnellstand vorhanden")
		return
	if core.load_state(ProjectSettings.globalize_path(QUICK)):
		key_queue.clear()
		world.visible = false   # rebuilt from the loaded map on the next frame
		_set_paused(false)
		_toast("Schnellstand geladen")

func _show_error(msg: String) -> void:
	mode = Mode.ERROR
	menu_status.text = msg
	menu_status.add_theme_color_override("font_color", Color(1, 0.7, 0.6))
	for c in menu_box.get_children():
		c.queue_free()
	menu_box.add_child(_button(T("Beenden"), func(): get_tree().quit()))

# ---------------------------------------------------------------- flow

func _enter_menu() -> void:
	mode = Mode.MENU
	menu_root.visible = true
	pause_panel.visible = false
	screen_rect.visible = false
	world.visible = false
	tilt_rect.visible = false
	fade_rect.color.a = 0.0
	menu_built_mask = -1
	menu_status.text = T("Lade …")
	for c in menu_box.get_children():
		c.queue_free()

func _start_game(bit: int) -> void:
	if bit >= 0:
		core.choose(bit)
	mode = Mode.GAME
	menu_root.visible = false
	settings_panel.visible = false
	screen_rect.visible = true
	acc = 0.0

func _to_menu() -> void:
	_set_paused(false)
	core.flush()
	core.reset_to_menu()
	_enter_menu()

func _quit() -> void:
	if core:
		core.flush()
	get_tree().quit()

func _open_settings() -> void:
	settings_panel.visible = true
	pause_panel.visible = false
	if mode == Mode.MENU:
		menu_box.get_parent().get_parent().visible = false   # keep the backdrop
	else:
		menu_root.visible = false
	# keep the whole window on screen on small displays
	var need := settings_panel.get_combined_minimum_size()
	var have := get_viewport().get_visible_rect().size * 0.96
	var k := minf(1.0, minf(have.x / need.x, have.y / need.y))
	settings_panel.pivot_offset = settings_panel.size * 0.5
	settings_panel.scale = Vector2(k, k)
	var first := settings_panel.get_child(0).get_child(1).get_child(0).get_child(0) as Control
	first.call_deferred("grab_focus")

func _close_settings() -> void:
	settings_panel.visible = false
	if mode == Mode.MENU:
		menu_root.visible = true
		menu_box.get_parent().get_parent().visible = true
		_rebuild_menu_items(menu_built_mask)
	else:
		pause_panel.visible = paused
		if paused:
			(pause_panel.get_child(0).get_child(1) as Control).grab_focus()

func _set_paused(p: bool) -> void:
	paused = p
	pause_panel.visible = p
	if debug_pause_btn:
		debug_pause_btn.visible = settings.debug
	settings_panel.visible = false
	if p:
		(pause_panel.get_child(0).get_child(1) as Control).call_deferred("grab_focus")

func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST and core:
		core.flush()

# ---------------------------------------------------------------- input

func _input(ev: InputEvent) -> void:
	if mode == Mode.LAUNCH:
		return
	if ev is InputEventKey and ev.pressed and not ev.echo:
		if ev.keycode == KEY_ESCAPE and personal_state == 1 and p_popup.visible:
			_close_popup()
			get_viewport().set_input_as_handled()
			return
		# Esc: back, like a PC game; on the free map (nothing to cancel) it opens the pause menu
		if ev.keycode == KEY_ESCAPE and mode == Mode.GAME and not paused and not settings_panel.visible and personal_state == 0:
			var free_map: bool = core.mouse_mode() == "map" and not core.cells().has(1) and not core.cells().has(2)
			if not free_map:
				key_queue.append([GBA_B, 4])
				get_viewport().set_input_as_handled()
				return
		if ev.keycode == KEY_ESCAPE and chapter_panel and chapter_panel.visible:
			_close_chapters()
			get_viewport().set_input_as_handled()
			return
		if ev.keycode == KEY_F5 and not paused:
			_quick_save()
			get_viewport().set_input_as_handled()
			return
		if ev.keycode == KEY_F8:
			_quick_load()
			get_viewport().set_input_as_handled()
			return
		if (ev.keycode == KEY_P or ev.keycode == KEY_PAUSE) and mode == Mode.GAME and personal_state == 0 and not settings_panel.visible:
			_set_paused(not paused)
			get_viewport().set_input_as_handled()
			return
		if ev.keycode == KEY_ESCAPE or ev.keycode == KEY_F1:
			if mode == Mode.GAME:
				if settings_panel.visible:
					_close_settings()
				else:
					_set_paused(not paused)
				get_viewport().set_input_as_handled()
			elif mode == Mode.MENU and settings_panel.visible:
				_close_settings()
			elif mode == Mode.SLOTS and slots_built and not picking:
				_slots_back()
		elif ev.keycode == KEY_F11 or (ev.keycode == KEY_ENTER and ev.alt_pressed):
			settings.fullscreen = not settings.fullscreen
			_apply_all()
	if mode == Mode.SLOTS and slots_built and not picking and ev is InputEventMouseButton and ev.pressed and ev.button_index == MOUSE_BUTTON_RIGHT:
		_slots_back()
	if mode == Mode.GAME and not paused and ev is InputEventMouseButton:
		if ev.button_index == MOUSE_BUTTON_MIDDLE:
			# middle button: drag = rotate/tilt the camera, short click = map overview
			if ev.pressed:
				mid_drag = 0.0
				mid_down = true
			else:
				if mid_down and mid_drag < 6.0:
					mouse_middle = true
				mid_down = false
		elif ev.pressed:
			var on_map: bool = world.visible and core.mouse_mode() == "map"
			match ev.button_index:
				MOUSE_BUTTON_LEFT: mouse_left = true
				MOUSE_BUTTON_RIGHT: mouse_right = true
				MOUSE_BUTTON_WHEEL_UP:
					if ev.ctrl_pressed or on_map:
						world.zoom = clampf(world.zoom * 0.88, 0.3, world.max_zoom())
					else:
						wheel += 1
				MOUSE_BUTTON_WHEEL_DOWN:
					if ev.ctrl_pressed or on_map:
						world.zoom = clampf(world.zoom * 1.12, 0.3, world.max_zoom())
					else:
						wheel -= 1
	if mode == Mode.GAME and not paused and ev is InputEventMouseMotion and mid_down and world.visible:
		mid_drag += ev.relative.length()
		if ev.shift_pressed:
			world.pan_by(ev.relative)   # Shift: move the camera
		else:
			world.yaw -= ev.relative.x * 0.35
			world.pitch_deg = clampf(world.pitch_deg + ev.relative.y * 0.25, 15.0, 89.0)
	if mode == Mode.GAME and not paused and ev is InputEventKey and ev.pressed and ev.keycode == KEY_HOME:
		world.reset_view()

## Camera keys (held): Z/X rotate, R/F tilt, +/- zoom.
func _camera_keys(delta: float) -> void:
	if not world.visible:
		return
	if Input.is_key_pressed(KEY_Z):
		world.yaw += 90.0 * delta
	if Input.is_key_pressed(KEY_X):
		world.yaw -= 90.0 * delta
	if Input.is_key_pressed(KEY_R):
		world.pitch_deg = clampf(world.pitch_deg + 45.0 * delta, 15.0, 89.0)
	if Input.is_key_pressed(KEY_F):
		world.pitch_deg = clampf(world.pitch_deg - 45.0 * delta, 15.0, 89.0)
	if Input.is_key_pressed(KEY_EQUAL) or Input.is_key_pressed(KEY_KP_ADD) or Input.is_key_pressed(KEY_PLUS):
		world.zoom = clampf(world.zoom * (1.0 - 1.2 * delta), 0.3, world.max_zoom())
	if Input.is_key_pressed(KEY_MINUS) or Input.is_key_pressed(KEY_KP_SUBTRACT):
		world.zoom = clampf(world.zoom * (1.0 + 1.2 * delta), 0.3, world.max_zoom())

func _held_keys() -> int:
	var k := 0
	for key in KEYMAP:
		if Input.is_key_pressed(key):
			k |= KEYMAP[key]
	for pad in Input.get_connected_joypads():
		for b in PADMAP:
			if Input.is_joy_button_pressed(pad, b):
				k |= PADMAP[b]
		var ax := Input.get_joy_axis(pad, JOY_AXIS_LEFT_X)
		var ay := Input.get_joy_axis(pad, JOY_AXIS_LEFT_Y)
		if ax < -0.5: k |= GBA_LEFT
		if ax > 0.5: k |= GBA_RIGHT
		if ay < -0.5: k |= GBA_UP
		if ay > 0.5: k |= GBA_DOWN
	# camera turned: arrow keys follow the screen (nearest quarter turn)
	if world.visible:
		var q := posmod(int(round(world.yaw / 90.0)), 4)
		for i in q:
			var nk := k & ~(GBA_UP | GBA_DOWN | GBA_LEFT | GBA_RIGHT)
			if k & GBA_UP: nk |= GBA_LEFT
			if k & GBA_LEFT: nk |= GBA_DOWN
			if k & GBA_DOWN: nk |= GBA_RIGHT
			if k & GBA_RIGHT: nk |= GBA_UP
			k = nk
	# opposite directions cancel
	if k & GBA_LEFT and k & GBA_RIGHT: k &= ~(GBA_LEFT | GBA_RIGHT)
	if k & GBA_UP and k & GBA_DOWN: k &= ~(GBA_UP | GBA_DOWN)
	return k

# ---------------------------------------------------------------- frame loop

func _layout() -> void:
	var vs := get_viewport().get_visible_rect().size
	var motes := menu_root.get_node_or_null("Motes")
	if motes:
		motes.position = vs * 0.5
		motes.emission_rect_extents = vs * 0.5
	if core:
		var e := int(round((vs.x / vs.y * 160.0 - 240.0) / 2.0))
		core.set_ext(clampi(e, 0, 96))

func _run_tests() -> void:
	test_frame += 1
	while test_cmds.size() > 0 and int(test_cmds[0][0]) <= test_frame:
		var c: Array = test_cmds.pop_front()
		match c[1]:
			"shot":
				DirAccess.make_dir_recursive_absolute(shot_dir())
				get_viewport().get_texture().get_image().save_png(shot_dir() + "/%s.png" % c[2])
			"keys":
				test_keys = int(c[2])
				test_keys_until = test_frame + (int(c[3]) if c.size() > 3 else 4)
			"choose":
				_open_slots(int(c[2]))
			"slot":
				_pick_slot(int(c[2]), c.size() > 3)
			"load":
				core.load_state(c[2])
				_start_game(-1)
			"mouse":
				test_mouse = Vector2(float(c[2]), float(c[3]))
			"click":
				test_mouse = Vector2(float(c[2]), float(c[3]))
				mouse_left = true
			"clicktile":
				var wx := float(c[2]) + 0.5
				var wz := float(c[3]) + 0.5
				test_mouse = world.cam.unproject_position(Vector3(wx, world.height_at(wx, wz), wz))
				mouse_left = c.size() > 4
			"rclick":
				mouse_right = true
			"freeze":
				_set_paused(c[2] == "1")
				pause_panel.visible = false
			"rawshot":
				var im: Image = core.frame()
				if im:
					im.save_png(shot_dir() + "/%s.png" % c[2])
				var f := FileAccess.open(shot_dir() + "/%s.txt" % c[2], FileAccess.WRITE)
				f.store_string("mode %d slot_bit %d picking %s built %s in_slot %s busy %s panel %s root %s menus %s title %s\n" % [mode, slot_bit, picking, slots_built, core.in_slot_select(), core.busy(), slot_panel.visible, menu_root.visible, settings.menus, slot_title.text])
				f.close()
			"uiclick":
				# real mouse click at window pixel x, y (goes through the GUI)
				for pressed in [true, false]:
					var ev := InputEventMouseButton.new()
					ev.button_index = MOUSE_BUTTON_LEFT
					ev.pressed = pressed
					ev.position = Vector2(float(c[2]), float(c[3]))
					ev.global_position = ev.position
					Input.parse_input_event(ev)
			"set":
				settings[c[2]] = c[3] == "1"
				_apply_all()
			"save":
				core.save_state(c[2])
			"pname":
				p_name.text = c[2]
				_set_month(int(c[3]))
				_set_gender(int(c[4]))
				_personal_done()
			"popup":
				if c[2] == "m":
					_open_months()
				else:
					_open_genders()
			"cam":
				world.yaw = float(c[2])
				world.pitch_deg = float(c[3])
				world.zoom = float(c[4])
				if c.size() > 6:
					world.pan = Vector3(float(c[5]), 0, float(c[6]))
				world._yaw_s = world.yaw
				world._pitch_s = world.pitch_deg
				world._zoom_s = world.zoom
				if world.visible:
					world._update_camera(1.0)
			"dbgtrade":
				print("TRADE ", core.trade())
			"dbg":
				print("menus ", core.menus(), " talk ", core.talk(), " spr ", core.sprites(), " cursor ", core.cursor(), " pick ", world.pick(test_mouse if test_mouse != null else Vector2.ZERO), " mode ", core.mouse_mode(), " view ", core.view())
			"esc":
				_set_paused(not paused)
			"settings":
				_open_settings()
			"lang":
				settings.lang = c[2]
				_apply_all()
				_retranslate()
			"qs":
				_quick_save()
			"ql":
				_quick_load()
			"pan":
				world.pan_by(Vector2(float(c[2]), float(c[3])))
			"rkey":
				# a real key press through the input system: rkey:<name> (e.g. Right, Enter, Escape)
				for down in ([true, false] if c.size() < 4 else [c[3] == "1"]):
					var ke := InputEventKey.new()
					ke.keycode = OS.find_keycode_from_string(c[2])
					ke.physical_keycode = ke.keycode
					ke.pressed = down
					Input.parse_input_event(ke)
			"rmouse":
				# real mouse: rmouse:<tile x>:<tile y>:<button 1|2> (0 = only move)
				var rx := float(c[2]) + 0.5
				var rz := float(c[3]) + 0.5
				var rp: Vector2 = world.cam.unproject_position(Vector3(rx, world.height_at(rx, rz), rz))
				test_mouse = rp   # headless has no pointer: the position is injected, the buttons are real events
				var mm := InputEventMouseMotion.new()
				mm.position = rp
				mm.global_position = rp
				Input.parse_input_event(mm)
				if int(c[4]) > 0:
					for down in [true, false]:
						var mb := InputEventMouseButton.new()
						mb.button_index = int(c[4])
						mb.pressed = down
						mb.position = rp
						mb.global_position = rp
						Input.parse_input_event(mb)
			"cur":
				print("CUR ", core.cursor(), " menus ", core.menus().size(), " paused ", paused)
			"launchset":
				_launch_set(":".join(c.slice(2)))
			"launchplay":
				_launch_start()
			"wheel":
				# real wheel events through the input system: wheel:<steps> (negative = zoom out)
				for k in absi(int(c[2])):
					var we := InputEventMouseButton.new()
					we.button_index = MOUSE_BUTTON_WHEEL_UP if int(c[2]) > 0 else MOUSE_BUTTON_WHEEL_DOWN
					we.pressed = true
					we.position = get_viewport().get_visible_rect().size * 0.5
					we.global_position = we.position
					Input.parse_input_event(we)
				print("ZOOM ", world.zoom, " max ", world.max_zoom())
			"dbgshop":
				print("SHOP ", core.shop().get("text"))
			"terrscan":
				# first inner cell of every terrain id on this map: TERR <tag> id:x:y ...
				var tb: PackedByteArray = core.terrain(0)
				var ms: Vector2i = core.map_size()
				var seen := {}
				for yy in ms.y:
					for xx in ms.x:
						var id := tb[yy * ms.x + xx]
						var inner := xx > 1 and yy > 1 and xx < ms.x - 2 and yy < ms.y - 2
						if not seen.has(id) or (inner and not seen[id][2]):
							seen[id] = [xx, yy, inner]
				var out: String = "TERR " + str(c[2])
				for id in seen:
					out += " %d:%d:%d" % [id, seen[id][0], seen[id][1]]
				print(out)
			"focus":
				var vv: Vector2i = core.view()
				world.pan = Vector3(float(c[2]) + 0.5 - (vv.x + 120) / 16.0, 0, float(c[3]) + 0.5 - (vv.y + 80) / 16.0)
				world.cam_ready = false
			"dumpobj":
				var oi: Image = core.obj_region(0, 0, 240, 48, -3)
				if oi:
					oi.save_png(shot_dir() + "/obj_%s.png" % c[2])
				core.set_capture_layers(true)
			"dumplayers":
				for li in 4:
					var lim: Image = core.layer_image(li)
					if lim:
						lim.get_region(Rect2i(0, 0, 240, 48)).save_png(shot_dir() + "/lay_%s_%d.png" % [c[2], li])
				core.set_capture_layers(false)
			"msize":
				var mz: Vector2i = core.map_size()
				print("MSIZE %s %d %d" % [str(c[2]), mz.x, mz.y])
			"zoominfo":
				print("ZOOM ", world.zoom, " max ", world.max_zoom())
			"debugch":
				core.debug_chapter(int(c[2]))
				print("CH ", core.chapter(), " pending ", core.debug_chapter_pending())
			"chapters":
				_open_chapters()
			"pickch":
				_pick_chapter(int(c[2]))
			"sprinfo":
				var sd: PackedInt32Array = core.sprites()
				for k in sd.size() / 7:
					printerr("SP %d %d %d %d f%d" % [sd[k*7], sd[k*7+1], sd[k*7+2], sd[k*7+3], sd[k*7+6]])
				printerr("FRONT ", world.bld_front.size())
			"terrnames":
				for i in 0x42:
					printerr("TN %02x %s" % [i, core.text(0x58C + 0x55 + i)])
			"scanterr":
				for fn in DirAccess.get_files_at(c[2]):
					if not fn.ends_with(".st"):
						continue
					core.load_state(c[2] + "/" + fn)
					core.run(2, 0)
					var tr: PackedByteArray = core.terrain(0)
					var cnt := {}
					for v in tr:
						if v in [0x0B, 0x24, 0x30, 0x38]:
							cnt[v] = int(cnt.get(v, 0)) + 1
					if not cnt.is_empty():
						var ms: Vector2i = core.map_size()
						var gi := tr.find(0x0B)
						printerr("ST %s %s ch%d gate %d,%d view %s" % [fn, str(cnt), core.chapter(), gi % maxi(ms.x, 1), gi / maxi(ms.x, 1), str(core.view())])
			"noui":
				for n in find_children("*", "CanvasLayer", true, false):
					(n as CanvasLayer).visible = false
			"chtour":
				_chapter_tour(int(c[2]), int(c[3]))
			"chlist":
				for i in 0x60:
					printerr("T %02x %s" % [i, core.chapter_title(i)])
			"chinfo":
				print("CH ", core.chapter(), " pending ", core.debug_chapter_pending(), " ", core.chapter_title(core.chapter()))
			"quit":
				get_tree().quit()
	if test_frame >= test_keys_until:
		test_keys = 0

func _process(delta: float) -> void:
	if fps_label and fps_label.visible:
		fps_label.text = "%d FPS" % Engine.get_frames_per_second()
	if toast and toast_t > 0.0:
		toast_t -= delta
		toast.modulate.a = clampf(toast_t * 2.0, 0.0, 1.0)
	if touring:
		return
	if mode == Mode.LAUNCH and (test_cmds.size() > 0 or test_frame > 0):
		_run_tests()
		return
	if core == null or mode == Mode.ERROR or mode == Mode.LAUNCH:
		return
	if test_cmds.size() > 0 or test_frame > 0:
		_run_tests()
		if mode == Mode.GAME and not paused and _personal_update():
			core.run(2 if core.busy() else 1, 0)
			core.audio()
			return
		if mode == Mode.GAME and not paused:
			_feed_mouse()
			core.run(1, test_keys | _queued_keys() | _held_keys())
			_present(1.0 / 60.0)
			core.audio()
			if settings.menus and core.in_slot_select():
				_open_save_slots()
			return
	_layout()
	if mode == Mode.MENU:
		_process_menu()
		return
	if mode == Mode.SLOTS:
		_process_slots()
		return
	if paused:
		_drain_audio()
		return
	if _personal_update():
		core.run(2 if core.busy() else 1, 0)
		_drain_audio()
		return
	_camera_keys(delta)
	var fast := Input.is_key_pressed(KEY_CTRL)
	var ff := float(settings.ffwd)
	acc += delta * FPS * (ff if fast else 1.0)
	var n := int(acc)
	var cap := int(4 * ff) if fast else 4
	if n > cap:
		n = cap
		acc = 0.0
	else:
		acc -= n
	if n > 0:
		_feed_mouse()
		core.run(n, _held_keys() | _queued_keys())
		_present(delta)
		if settings.menus and core.in_slot_select():
			_open_save_slots()
		elif core.menu_mask() != 0 and not core.busy():
			# the game went back to its main menu (e.g. after the game over screen)
			_enter_menu()
	_drain_audio(fast)

func _process_menu() -> void:
	if core.busy():
		core.run(30, 0)
		_drain_audio(true)
		return
	var mask: int = core.menu_mask()
	if mask != menu_built_mask:
		menu_built_mask = mask
		menu_status.text = ""
		_rebuild_menu_items(mask)
		core.run(1, 0)
		var img: Image = core.frame()
		if img:
			var bg := img.duplicate()
			# blurred backdrop from the game's own title background
			bg.resize(img.get_width() / 12, img.get_height() / 12, Image.INTERPOLATE_BILINEAR)
			for k in 4:
				bg.resize(bg.get_width() * 2, bg.get_height() * 2, Image.INTERPOLATE_BILINEAR)
			menu_bg.texture = ImageTexture.create_from_image(bg)
	else:
		core.run(1, 0)
	_drain_audio()

func _drain_audio(skip := false) -> void:
	var a: PackedVector2Array = core.audio()
	if skip:
		return
	var free := playback.get_frames_available()
	if a.size() > free:
		a = a.slice(a.size() - free)
	if a.size() > 0:
		playback.push_buffer(a)

var key_queue: Array = []  # [gba keys, frames]
var mid_down := false
var mid_drag := 0.0

func _queued_keys() -> int:
	if key_queue.is_empty():
		return 0
	var k: int = key_queue[0][0]
	key_queue[0][1] -= 1
	if key_queue[0][1] <= 0:
		key_queue.pop_front()
	return k

## Frame position inside a natively drawn window (its GBA pixels are transparent).
func _over_native(p: Vector2) -> bool:
	if menu_ui.visible:
		for m in menu_ui.menus:
			if Rect2(m.rect).grow(2).has_point(p):
				return true
		if not menu_ui.help.is_empty() and Rect2(menu_ui.help.rect).has_point(p):
			return true
	if talk_ui.visible and not talk_ui.data.is_empty() and Rect2(talk_ui.data.rect).has_point(p):
		return true
	return false

func _feed_mouse() -> void:
	if status_root.visible != status_bg_on:
		status_bg_on = status_root.visible
		core.set_capture_layers(status_bg_on)
		if not status_bg_on:
			status_bg.texture = null
	if status_bg_on and Engine.get_process_frames() % 2 == 0:
		var bgi: Image = core.layer_image(3)
		if bgi and not bgi.is_invisible():
			# a screen without its own picture keeps the marble
			if status_bg.texture is ImageTexture and (status_bg.texture as ImageTexture).get_size() == Vector2(bgi.get_size()):
				(status_bg.texture as ImageTexture).update(bgi)
			else:
				status_bg.texture = ImageTexture.create_from_image(bgi)
		else:
			status_bg.texture = null
	if status_root.visible:
		core.mouse_none()
		core.mouse_buttons(false, false, false, 0)
		mouse_left = false
		mouse_right = false
		mouse_middle = false
		wheel = 0
		return
	# yes/no inside a speech bubble: click an answer
	if mouse_left and talk_ui.visible and talk_ui.data.get("choice", -1) >= 0:
		var lp := talk_ui.get_local_mouse_position()
		for k in 2:
			var c: Label = talk_ui.choice_lbl[k]
			if Rect2(c.position - Vector2(20, 4), c.size + Vector2(24, 8)).has_point(lp):
				var cur: int = talk_ui.data.choice
				if k != cur:
					key_queue.append([GBA_LEFT if k == 0 else GBA_RIGHT, 3])
					key_queue.append([0, 3])
				key_queue.append([GBA_A, 3])
				mouse_left = false
	var mp := get_viewport().get_mouse_position()
	if test_mouse != null:
		mp = test_mouse
	var r := screen_rect.get_global_rect()
	var fw: int = core.frame_width()
	var fx := (mp.x - r.position.x) / r.size.x * fw
	var fy := (mp.y - r.position.y) / r.size.y * 160.0
	var inside := fx >= 0 and fy >= 0 and fx < fw and fy < 160
	var over_ui := inside
	if world.visible and inside and screen_img != null:
		over_ui = screen_img.get_pixel(int(fx), int(fy)).a > 0.05 or _over_native(Vector2(fx, fy))
	if over_ui:
		core.mouse_ui(fx, fy)
	elif world.visible:
		var p = world.pick(mp)
		if p == null:
			core.mouse_none()
		else:
			core.mouse_map(p.x, p.y)
	else:
		core.mouse_none()
	core.mouse_buttons(mouse_left, mouse_right, mouse_middle, wheel)
	mouse_left = false
	mouse_right = false
	mouse_middle = false
	wheel = 0

func _present(delta: float) -> void:
	if _status_update():
		return   # the status screen covers everything (the map stays built underneath)
	var hd: bool = settings.hd and core.is_map()
	if hd and not world.visible:
		world.visible = true
		world.rebuild(core)
	elif not hd and world.visible:
		world.visible = false
		tilt_rect.visible = false
	if hd:
		var sz: Vector2i = core.map_size()
		if sz.x != world.map_w or sz.y != world.map_h:
			world.rebuild(core)
		world.update_frame(delta)
		tilt_rect.visible = settings.dof and int(settings.quality) > 0
		var f: int = core.fade()
		fade_rect.color = Color(0, 0, 0, -f / 16.0) if f < 0 else Color(1, 1, 1, f / 16.0)
	else:
		fade_rect.color.a = 0.0
	screen_img = core.frame()
	if screen_img == null:
		return
	if screen_tex.get_width() == screen_img.get_width() and screen_tex.get_height() == screen_img.get_height():
		screen_tex.update(screen_img)
	else:
		screen_tex.set_image(screen_img)
	(screen_rect.material as ShaderMaterial).set_shader_parameter("tex_size", Vector2(screen_img.get_width(), 160))
	var vs := get_viewport().get_visible_rect().size
	var scale := vs.y / 160.0
	var w := screen_img.get_width() * scale
	if w > vs.x:
		scale = vs.x / screen_img.get_width()
		w = vs.x
	var h := 160.0 * scale
	screen_rect.position = Vector2((vs.x - w) * 0.5, (vs.y - h) * 0.5)
	screen_rect.size = Vector2(w, h)
	var td: Dictionary = core.talk()
	if td.get("choice", -1) >= 0 and talk_ui.hand == null:
		var hi: Image = core.obj_image(0, 0, 16, 16)
		if hi:
			talk_ui.hand = ImageTexture.create_from_image(hi)
	talk_ui.show_talk(td)
	var mi: Dictionary = core.mapinfo()
	var bd: Dictionary = core.battle() if settings.menus else {}
	var il: Texture2D = null
	var ir: Texture2D = null
	if bd.has("sides"):
		for k in 2:
			var sd: Dictionary = bd.sides[k]
			if sd.has("icon") and battle_ui.icons[k] == null:
				var ic: PackedInt32Array = sd.icon
				var img: Image = core.obj_image(ic[0], ic[1], 16, 16)
				if img and not img.is_invisible():
					if k == 0:
						il = ImageTexture.create_from_image(img)
					else:
						ir = ImageTexture.create_from_image(img)
	battle_ui.show_battle(bd, il, ir)
	var ld: Dictionary = core.levelup() if settings.menus else {}
	var face: Texture2D = null
	if ld.is_empty():
		lv_face_frames = 0
	else:
		lv_face_frames += 1
		# the portrait is made of objects (palette 15) at the right; take it once it has settled
		if lv_face_frames == 40:
			var reg: Image = core.obj_region(140, 80, 96, 80, 15)
			if reg and not reg.is_invisible():
				face = ImageTexture.create_from_image(reg)
	levelup_ui.show_levelup(ld, face)
	banner_ui.show_banner(core.banner())
	trade_ui.show_trade(core.trade() if settings.menus else {})
	var shd: Dictionary = core.shop() if settings.menus else {}
	var sface: Texture2D = null
	if shd.is_empty():
		shop_face_frames = 0
	else:
		shop_face_frames += 1
		if shop_face_frames % 30 == 2:
			var fi: Image = core.obj_region(8, 8, 48, 48, 6)
			if fi and not fi.is_invisible():
				sface = ImageTexture.create_from_image(fi)
	# the shop's own background picture (the game's lowest layer)
	if shd.is_empty() == shop_bg_on:
		shop_bg_on = not shd.is_empty()
		if not status_bg_on:
			core.set_capture_layers(shop_bg_on)
		if not shop_bg_on:
			shop_ui.bg = null
	if shop_bg_on and shop_face_frames % 4 == 3:
		var sbg: Image = core.layer_image(3)
		if sbg and not sbg.is_invisible():
			shop_ui.bg = ImageTexture.create_from_image(sbg)
	shop_ui.yes_label = T("Ja")
	shop_ui.no_label = T("Nein")
	shop_ui.show_shop(shd, sface, core.ext_left())
	narr_ui.show_lines(core.worldmap() if settings.menus else PackedStringArray(), core.ext_left())
	prepbar_ui.labels = {"Fight!": T("Zum Kampf!"), "Back": T("Zurück"), "Info": T("Info"), "Menu": T("Menü")}
	minimap_ui.show_map(core.minimap() if settings.menus and mode == Mode.GAME else {}, world.map_tex, world.MARGIN, T("Kartenübersicht"))
	prepbar_ui.show_state(core.prepmap() if settings.menus and mode == Mode.GAME and absi(core.fade()) < 10 else 0)
	mapinfo_ui.show_info(mi, core.bg0_image() if mi.has("unit") else null)
	var ms: Array = core.menus()
	var fd: Dictionary = core.forecast()
	menu_ui.fc = fd
	menu_ui.rhelp = core.rhelp() if settings.menus else {}
	menu_ui.mapexp = core.mapexp() if settings.menus else {}
	menu_ui.help_label = T("Hilfe")
	var gw: Array = core.generic_windows() if settings.menus else []
	if levelup_ui.visible:
		gw = []   # the level-up card is drawn by its own screen
	var gobj := []
	for gwin in gw:
		var sr: Rect2i = gwin.src
		gobj.append(core.obj_region(sr.position.x, sr.position.y, sr.size.x, sr.size.y, -2))
	menu_ui.set_generic(gw, core.bg0_image() if not gw.is_empty() else null, gobj)
	if ms.is_empty() and fd.is_empty():
		menu_ui.show_menus(ms, null, null)
	else:
		menu_ui.show_menus(ms, core.bg0_image(), core.obj_image(0, 0, 16, 16), core.help())
