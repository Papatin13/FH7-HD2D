#!/bin/sh
# Starts the game. On the first start the launcher asks for your ROM.
cd "$(dirname "$0")"
exec ./engine/godot --path ./game "$@"
