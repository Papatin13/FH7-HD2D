#!/bin/sh
# Startet das Spiel. Beim ersten Start fragt der Launcher nach deinem ROM.
cd "$(dirname "$0")"
exec ./engine/godot --path ./spiel "$@"
