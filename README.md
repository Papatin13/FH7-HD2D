# FE7-HD2D

Ein inoffizielles HD-2D-Remake von *Fire Emblem* (Game Boy Advance, US-Version) für den PC:
die Karten als 3D-Diorama auf einem Kartentisch im Kriegsraum, scharf neu gezeichnete Menüs
im Stil des Originals und volle Steuerung mit **Maus und Tastatur**.

> **Dieses Repository enthält keine Spieldaten.** Grafiken, Texte, Musik und Spiellogik werden
> zur Laufzeit aus dem ROM des Spielers gelesen. Du brauchst dein eigenes ROM der US-Version
> (z. B. eine Sicherungskopie deines Moduls).
>
> Nicht mit Nintendo oder Intelligent Systems verbunden. „Fire Emblem“ ist eine Marke von Nintendo.

*English: unofficial HD-2D remake of Fire Emblem (GBA, US) for PC with mouse and keyboard
controls, built with Rust and Godot 4.5. No game data is included – it runs from your own ROM.*

## Spielen

1. Die neueste Version unter **Releases** herunterladen und entpacken (Linux x86-64).
2. `./start.sh` ausführen.
3. Im Launcher das ROM auswählen oder ins Fenster ziehen. Geprüft wird auf das unveränderte
   US-ROM (Spielcode `AE7E`, CRC32 `2A524221`). Danach startet das Spiel direkt.

Der Spielstand (`.sav`) liegt neben dem ROM. Einstellungen: `~/.local/share/godot/app_userdata/FE7-HD2D/`.

### Steuerung

| | |
|---|---|
| Maus | alles anklicken (Karte, Menüs, Listen), Rechtsklick = zurück |
| Mausrad | in Menüs blättern, auf der Karte zoomen |
| Mittlere Maustaste ziehen | Kamera drehen (mit Shift: verschieben) |
| Pfeile / WASD | bewegen |
| Enter / Leertaste | bestätigen |
| Esc / Rücktaste | zurück, auf freier Karte Pause |
| Q / Tab | nächste Einheit |
| E / I | Einheiteninfo |
| F5 / F8 | Schnellspeichern / Schnellladen |
| F11 | Vollbild |
| Strg halten | Vorspulen |

## Selbst bauen

Benötigt: [Rust](https://rustup.rs) (stable) und [Godot 4.5.1](https://godotengine.org/download/archive/4.5.1-stable/).

```sh
tools/build.sh                     # baut den Spielkern nach godot/bin/
godot --path godot                 # startet das Spiel aus dem Quelltext
```

Optional schneller: `tools/build.sh <dein-rom.gba>` übersetzt den Code deines ROMs nach Rust
(`core/generated/`) und bindet ihn ein. **Solche Builds enthalten Spielcode und dürfen nicht
weitergegeben werden** – `core/generated/` ist deshalb von git ausgeschlossen, und
`tools/package.sh` verweigert solche Builds. Ohne ROM-Code läuft alles über den Interpreter,
mit identischem Spielverhalten (> 500 Bilder/s auf einem normalen PC).

Release-Paket bauen:

```sh
tools/build.sh
tools/package.sh /pfad/zu/Godot_v4.5.1-stable_linux.x86_64 1.0.0   # -> dist/
```

Ein Git-Tag `v*` baut das Paket auch automatisch über GitHub Actions und hängt es an ein Release.

## Aufbau

```
core/            Spielkern in Rust (Cargo-Workspace)
  crates/gba     GBA-Plattform: CPU (ARM/Thumb), Grafik, Ton, DMA, Timer, BIOS-Ersatz
  crates/fe7hd   GDExtension (Klasse Fe7Core) – Schnittstelle zum Godot-Projekt
  crates/recomp  statischer Rekompiler ROM -> Rust (nur für lokale Builds)
  crates/fe7gen  Platzhalter für den rekompilierten Code
  crates/fe7     eigenständiges 2D-Frontend zum Testen
  traces/        Blockadressen für den Rekompiler (keine Spieldaten)
godot/           Godot-Projekt: Skripte, Shader, Schriften
  ui/frames.json Bauplan der Menürahmen – nur Adressen im ROM, die Grafik kommt aus dem ROM
tools/           Bau- und Paketskripte, Vorlagen fürs Release
third_party/     Lizenzen der Fremdbestandteile (Godot, godot-rust, Schriften …)
```

Mehr zur Technik: [core/README.md](core/README.md).

### Entwicklung

Automatische Tests ohne Bildschirm (z. B. mit `xvfb-run`):

```sh
FE7HD_SHOTS=/tmp/shots FE7HD_TEST="20:launchset:/pfad/rom.gba;26:launchplay;300:shot:menu;301:quit" \
  godot --path godot --resolution 1280x720
```

`FE7HD_TEST` ist eine Liste `bild:befehl:argument`, `FE7HD_SHOTS` der Ordner für Bildschirmfotos,
`FE7HD_LAUNCHER=1` erzwingt den Launcher.

## Beitragen

Bitte niemals ROMs, Spielstände, Bildschirmfotos mit Spielgrafik, aus dem ROM erzeugten Code
oder andere Daten aus dem Spiel einchecken.

## Lizenz

Der eigene Code steht unter der [MIT-Lizenz](LICENSE). Fremdbestandteile stehen unter ihren eigenen
Lizenzen, siehe [third_party/UEBERSICHT.txt](third_party/UEBERSICHT.txt).
