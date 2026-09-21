#!/usr/bin/env python3
"""Render the Cornix Vial layout as a self-contained SVG."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from html import escape
import json
from pathlib import Path
import re
import tempfile
from typing import Any


MAX_USED_LAYERS = 5
EMPTY_KEYCODES = {"KC_NO", "KC_TRNS"}
KEY_WIDTH = 52
KEY_HEIGHT = 40
KEY_PITCH = 58
SVG_WIDTH = 1240
LAYER_HEIGHT = 314


@dataclass(frozen=True)
class LayerMapping:
    left_rows: tuple[tuple[str, ...], ...]
    right_rows: tuple[tuple[str, ...], ...]
    left_encoder: tuple[str, str, str]
    right_encoder: tuple[str, str, str]


DISPLAY_LABELS = {
    "KC_BSPACE": "Bksp",
    "KC_DELETE": "Del",
    "KC_ENTER": "Enter",
    "KC_ESCAPE": "Esc",
    "KC_GESC": "` / Esc",
    "KC_TAB": "Tab",
    "KC_SPACE": "Space",
    "KC_LCTRL": "L Ctrl",
    "KC_RCTRL": "R Ctrl",
    "KC_LSHIFT": "L Shift",
    "KC_RSHIFT": "R Shift",
    "KC_LALT": "L Alt",
    "KC_RALT": "R Alt",
    "KC_LGUI": "L GUI",
    "KC_RGUI": "R GUI",
    "KC_UP": "↑",
    "KC_DOWN": "↓",
    "KC_LEFT": "←",
    "KC_RIGHT": "→",
    "KC_BSLASH": "\\",
    "KC_SLASH": "/",
    "KC_DOT": ".",
    "KC_COMMA": ",",
    "KC_SCOLON": ";",
    "KC_MINUS": "-",
    "KC_EQUAL": "=",
    "KC_QUOTE": "'",
    "KC_GRAVE": "`",
    "KC_LBRACKET": "[",
    "KC_RBRACKET": "]",
    "KC_JYEN": "¥",
    "KC_MUTE": "Mute",
    "KC_VOLU": "Vol +",
    "KC_VOLD": "Vol −",
    "KC_WH_L": "Wheel ←",
    "KC_WH_R": "Wheel →",
    "KC_WH_U": "Wheel ↑",
    "KC_WH_D": "Wheel ↓",
    "KC_MS_L": "Mouse ←",
    "KC_MS_R": "Mouse →",
    "KC_MS_U": "Mouse ↑",
    "KC_MS_D": "Mouse ↓",
    "KC_BTN1": "Mouse 1",
    "KC_BTN2": "Mouse 2",
    "KC_BTN3": "Mouse 3",
    "KC_WBAK": "Back",
    "KC_WFWD": "Forward",
    "KC_WREF": "Reload",
    "KC_NO": "—",
    "KC_TRNS": "▽",
}


def _as_keycode(value: object) -> str:
    return "KC_NO" if value == -1 else str(value)


def _layers(vil: dict[str, Any]) -> list[list[list[object]]]:
    layers = vil.get("layout")
    if not isinstance(layers, list):
        raise ValueError("Vial data must contain a layout array")
    for layer_index, layer in enumerate(layers):
        if not isinstance(layer, list) or len(layer) != 8:
            raise ValueError(f"Layer {layer_index} must contain 8 Cornix rows")
        for row_index, row in enumerate(layer):
            if not isinstance(row, list) or len(row) < 7:
                raise ValueError(
                    f"Layer {layer_index}, row {row_index} must contain 7 slots"
                )
    return layers


def _layer_is_used(layer: list[list[object]]) -> bool:
    # Vial repeats encoder presses and encoder rotations in otherwise empty
    # exported layers. The first six matrix slots are the 48 physical keys.
    return any(
        _as_keycode(row[column]) not in EMPTY_KEYCODES
        for row in layer
        for column in range(6)
    )


def used_layer_indexes(vil: dict[str, Any]) -> list[int]:
    indexes = [
        index for index, layer in enumerate(_layers(vil)) if _layer_is_used(layer)
    ]
    if len(indexes) > MAX_USED_LAYERS:
        raise ValueError(
            f"The README diagram supports at most {MAX_USED_LAYERS} in-use layers; "
            f"found {len(indexes)} ({', '.join(map(str, indexes))})"
        )
    return indexes


def map_layer(vil: dict[str, Any], layer_index: int) -> LayerMapping:
    layer = _layers(vil)[layer_index]
    encoders = vil.get("encoder_layout")
    try:
        encoder_layer = encoders[layer_index]
        left_rotation = encoder_layer[0]
        right_rotation = encoder_layer[1]
        if len(left_rotation) != 2 or len(right_rotation) != 2:
            raise IndexError
    except (IndexError, TypeError):
        raise ValueError(f"Layer {layer_index} must define two Cornix encoders") from None

    left_rows = tuple(
        tuple(_as_keycode(value) for value in row[:6]) for row in layer[:4]
    )
    right_rows = tuple(
        tuple(_as_keycode(value) for value in reversed(row[:6]))
        for row in layer[4:]
    )
    return LayerMapping(
        left_rows=left_rows,
        right_rows=right_rows,
        left_encoder=(
            _as_keycode(left_rotation[0]),
            _as_keycode(left_rotation[1]),
            _as_keycode(layer[2][6]),
        ),
        right_encoder=(
            _as_keycode(right_rotation[0]),
            _as_keycode(right_rotation[1]),
            _as_keycode(layer[5][6]),
        ),
    )


def display_label(keycode: str) -> str:
    if keycode in DISPLAY_LABELS:
        return DISPLAY_LABELS[keycode]
    shifted = re.fullmatch(r"LSFT\((.+)\)", keycode)
    if shifted:
        return f"⇧ {display_label(shifted.group(1))}"
    layer_key = re.fullmatch(r"MO\((\d+)\)", keycode)
    if layer_key:
        return f"MO {layer_key.group(1)}"
    if re.fullmatch(r"KC_[A-Z0-9]", keycode) or re.fullmatch(r"KC_F\d{1,2}", keycode):
        return keycode.removeprefix("KC_")
    user_key = re.fullmatch(r"USER(\d+)", keycode)
    if user_key:
        return f"User {int(user_key.group(1))}"
    return keycode.removeprefix("KC_").replace("_", " ")


def _key_kind(keycode: str) -> str:
    if keycode in EMPTY_KEYCODES or keycode == "KC_NO":
        return "empty"
    if keycode.startswith(("MO(", "TG(", "TO(", "DF(", "LT(")):
        return "layer-key"
    if keycode.startswith("USER"):
        return "custom"
    if any(
        token in keycode
        for token in ("CTRL", "SHIFT", "ALT", "GUI", "LSFT(", "RSFT(")
    ):
        return "modifier"
    if keycode.startswith(("KC_MS_", "KC_WH_", "KC_BTN")) or keycode in {
        "KC_WBAK",
        "KC_WFWD",
        "KC_WREF",
    }:
        return "pointer"
    return "normal"


def _number(value: float) -> str:
    return str(int(value)) if value.is_integer() else f"{value:.1f}"


def _render_key(
    x: float,
    y: float,
    keycode: str,
    slot: str,
    rotation: float = 0,
) -> list[str]:
    center_x = x + KEY_WIDTH / 2
    center_y = y + KEY_HEIGHT / 2
    transform = ""
    if rotation:
        transform = (
            f' transform="rotate({_number(rotation)} {_number(center_x)} '
            f'{_number(center_y)})"'
        )
    label = escape(display_label(keycode))
    font_class = " key-label--small" if len(display_label(keycode)) > 8 else ""
    return [
        f'<g class="key {_key_kind(keycode)}" data-slot="{escape(slot)}"{transform}>',
        f"<title>{escape(keycode)}</title>",
        (
            f'<rect x="{_number(x)}" y="{_number(y)}" width="{KEY_WIDTH}" '
            f'height="{KEY_HEIGHT}" rx="8"/>'
        ),
        (
            f'<text class="key-label{font_class}" x="{_number(center_x)}" '
            f'y="{_number(center_y + 1)}">{label}</text>'
        ),
        "</g>",
    ]


def _render_half(
    mapping: LayerMapping,
    layer_y: float,
    side: str,
) -> list[str]:
    is_left = side == "left"
    rows = mapping.left_rows if is_left else mapping.right_rows
    start_x = 34 if is_left else 854
    top_y = layer_y + 48
    offsets = (12, 6, 0, 3, 8, 14) if is_left else (14, 8, 3, 0, 6, 12)
    result: list[str] = []

    for row_index in range(3):
        for column, keycode in enumerate(rows[row_index]):
            result.extend(
                _render_key(
                    start_x + column * KEY_PITCH,
                    top_y + row_index * 44 + offsets[column],
                    keycode,
                    f"{side}-r{row_index}-c{column}",
                )
            )

    if is_left:
        main_keys = enumerate(rows[3][:3])
        thumb_keys = enumerate(rows[3][3:])
        for column, keycode in main_keys:
            result.extend(
                _render_key(
                    start_x + column * KEY_PITCH,
                    top_y + 132 + offsets[column],
                    keycode,
                    f"left-r3-c{column}",
                )
            )
        for thumb, keycode in thumb_keys:
            result.extend(
                _render_key(
                    start_x + (thumb + 3) * KEY_PITCH - 12,
                    top_y + 188 + thumb * 5,
                    keycode,
                    f"left-thumb-{thumb}",
                    8 + thumb * 2,
                )
            )
    else:
        # map_layer has already mirrored the row: thumbs are first, outer keys last.
        for thumb, keycode in enumerate(rows[3][:3]):
            result.extend(
                _render_key(
                    start_x + thumb * KEY_PITCH + 12,
                    top_y + 198 - thumb * 5,
                    keycode,
                    f"right-thumb-{thumb}",
                    -12 + thumb * 2,
                )
            )
        for column, keycode in enumerate(rows[3][3:], start=3):
            result.extend(
                _render_key(
                    start_x + column * KEY_PITCH,
                    top_y + 132 + offsets[column],
                    keycode,
                    f"right-r3-c{column}",
                )
            )
    return result


def _render_encoder(
    x: float,
    y: float,
    name: str,
    mappings: tuple[str, str, str],
) -> list[str]:
    ccw, cw, press = mappings
    return [
        f'<g class="encoder" transform="translate({_number(x)} {_number(y)})">',
        f"<title>{escape(name)}: {escape(ccw)}, {escape(cw)}, {escape(press)}</title>",
        '<rect width="154" height="116" rx="12"/>',
        '<circle cx="77" cy="31" r="15"/>',
        f'<text class="encoder-name" x="77" y="62">{escape(name)}</text>',
        f'<text class="encoder-action" x="77" y="82">↺ {escape(display_label(ccw))}</text>',
        f'<text class="encoder-action" x="77" y="98">{escape(display_label(cw))} ↻</text>',
        f'<text class="encoder-press" x="77" y="111">Press: {escape(display_label(press))}</text>',
        "</g>",
    ]


def render_keymap(vil: dict[str, Any]) -> str:
    layer_indexes = used_layer_indexes(vil)
    if not layer_indexes:
        raise ValueError("No in-use keymap layers were found")
    height = 76 + len(layer_indexes) * LAYER_HEIGHT + 18
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        (
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{SVG_WIDTH}" '
            f'height="{height}" viewBox="0 0 {SVG_WIDTH} {height}" '
            'role="img" aria-labelledby="svg-title svg-description">'
        ),
        '<title id="svg-title">Cornix keymap</title>',
        (
            '<desc id="svg-description">Up to five in-use layers rendered from '
            'cornix/main.vil, including both encoders.</desc>'
        ),
        "<style>",
        "svg { background: #f8fafc; }",
        "text { font-family: Inter, ui-sans-serif, system-ui, sans-serif; fill: #172033; }",
        ".page-title { font-size: 24px; font-weight: 700; }",
        ".page-subtitle { font-size: 12px; fill: #64748b; }",
        ".layer-card { fill: #ffffff; stroke: #d7e0ea; stroke-width: 1.5; }",
        ".layer-title { font-size: 17px; font-weight: 700; }",
        ".layer-note { font-size: 11px; fill: #64748b; }",
        ".key rect { fill: #fffaf3; stroke: #94a3b8; stroke-width: 1.4; }",
        ".key.modifier rect { fill: #eef2ff; stroke: #818cf8; }",
        ".key.layer-key rect { fill: #ffedd5; stroke: #f97316; stroke-width: 2; }",
        ".key.pointer rect { fill: #ecfdf5; stroke: #34d399; }",
        ".key.custom rect { fill: #fdf2f8; stroke: #f472b6; }",
        ".key.empty rect { fill: #f1f5f9; stroke: #cbd5e1; stroke-dasharray: 3 3; }",
        ".key-label { dominant-baseline: middle; text-anchor: middle; font-size: 11px; font-weight: 650; }",
        ".key-label--small { font-size: 8.5px; }",
        ".encoder rect { fill: #f8fafc; stroke: #cbd5e1; }",
        ".encoder circle { fill: #ffedd5; stroke: #f97316; stroke-width: 2; }",
        ".encoder-name { text-anchor: middle; font-size: 11px; font-weight: 700; }",
        ".encoder-action { text-anchor: middle; font-size: 9px; fill: #475569; }",
        ".encoder-press { text-anchor: middle; font-size: 8px; fill: #64748b; }",
        "</style>",
        '<text class="page-title" x="24" y="34">Cornix keymap</text>',
        (
            '<text class="page-subtitle" x="24" y="55">Generated from '
            'cornix/main.vil · orange keys switch layers · hover a key for its raw code</text>'
        ),
    ]

    for position, layer_index in enumerate(layer_indexes):
        card_y = 72 + position * LAYER_HEIGHT
        mapping = map_layer(vil, layer_index)
        lines.extend(
            [
                f'<g id="layer-{layer_index}">',
                (
                    f'<rect class="layer-card" x="16" y="{card_y}" width="1208" '
                    'height="298" rx="16"/>'
                ),
                (
                    f'<text class="layer-title" x="34" y="{card_y + 28}">'
                    f"Layer {layer_index}</text>"
                ),
                (
                    f'<text class="layer-note" x="1204" y="{card_y + 27}" '
                    'text-anchor="end">48 keys + 2 encoders</text>'
                ),
            ]
        )
        lines.extend(_render_half(mapping, card_y, "left"))
        lines.extend(_render_half(mapping, card_y, "right"))
        lines.extend(
            _render_encoder(448, card_y + 92, "Left encoder", mapping.left_encoder)
        )
        lines.extend(
            _render_encoder(638, card_y + 92, "Right encoder", mapping.right_encoder)
        )
        lines.append("</g>")

    lines.append("</svg>")
    return "\n".join(lines) + "\n"


def write_svg(input_path: Path, output_path: Path) -> None:
    with input_path.open(encoding="utf-8") as source:
        vil = json.load(source)
    svg = render_keymap(vil)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    temporary_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            "w",
            encoding="utf-8",
            dir=output_path.parent,
            prefix=f".{output_path.name}.",
            suffix=".tmp",
            delete=False,
        ) as destination:
            destination.write(svg)
            temporary_path = Path(destination.name)
        temporary_path.replace(output_path)
    finally:
        if temporary_path is not None and temporary_path.exists():
            temporary_path.unlink()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Render a Cornix Vial .vil file as an SVG keymap diagram."
    )
    parser.add_argument("input", type=Path, help="input .vil file")
    parser.add_argument("output", type=Path, help="output .svg file")
    args = parser.parse_args()
    write_svg(args.input, args.output)


if __name__ == "__main__":
    main()
