# Spielkern (Rust)

Der Kern führt das Spiel aus dem ROM des Spielers aus und stellt dem Godot-Projekt den
Spielzustand bereit. Gebaut wird er über `tools/build.sh` im Hauptordner.

## Wie es funktioniert

```
crates/
  gba/      Plattformschicht: Speicher, CPU-Semantik (ARM/Thumb), Software-PPU (Modi 0–5, Affine,
            Sprites, Fenster, Blending, Mosaik), DMA, Timer, IRQ, BIOS-Ersatz (HLE),
            Sound (Direct Sound FIFO + 4 PSG-Kanäle), Savestates
  recomp/   Statischer Rekompiler: übersetzt den Maschinencode des ROMs in Rust-Quelltext
  fe7gen/   Platzhalter; mit tools/build.sh <rom> bindet er den aus DEINEM ROM erzeugten Code ein
            (core/generated/: ~60 000 Basisblöcke / ~317 000 Instruktionen, nie hochladen)
  fe7hd/    GDExtension für das Godot-Projekt (Klasse Fe7Core): Spielzustand, Karten, Texte, Rahmen
  fe7/      Eigenständiges 2D-Frontend (Fenster, Tastatur, Maus, Audio) – nützlich zum Testen
traces/     Blockadressen aus Spielabläufen, mit denen der Rekompiler Code findet (keine Spieldaten)
```

Der Rekompiler findet Code per rekursivem Abstieg ab dem Reset-Vektor, über Funktionszeiger im ROM
und über Spuren echter Spielabläufe. Jeder Basisblock wird zu einer Rust-Funktion, die dieselben
Instruktions-Helfer wie der Referenz-Interpreter aufruft (Operanden schon dekodiert, Literal-Pools
als Konstanten eingefaltet). Code, den das Spiel zur Laufzeit ins IWRAM kopiert (Sound-Mixer,
IRQ-Handler), wird ebenfalls rekompiliert und vor jeder Ausführung gegen den RAM-Inhalt geprüft.
Was nicht rekompiliert ist, läuft über den Interpreter – das Spiel bleibt so immer lauffähig.

Prüfung: `--verify` führt jeden rekompilierten Block aus, macht ihn rückgängig, führt dieselben
Instruktionen im Interpreter aus und vergleicht Register und jeden Speicherschreibzugriff.
Stand der Tests: >240 Mio. Blockausführungen (Intro, Titel, Spielstart, Prolog, Kampf) ohne Abweichung.

Grafik, Musik und Texte liest das Programm zur Laufzeit aus dem ROM. Der erzeugte Rust-Code stammt
ebenfalls aus dem ROM und liegt deshalb nur lokal in `core/generated/` (von git ignoriert).

## Entwickler-Optionen des 2D-Frontends

```
cargo run --release -p fe7 -- <rom.gba>
```

`--headless --frames N --script "120:START,200:A" --shot-every N --out dir`, `--verify`, `--interp`,
`--trace datei.txt` (sammelt Blockadressen für den Rekompiler), `--save/--load`.
