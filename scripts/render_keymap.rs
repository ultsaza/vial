//! Render the Cornix Vial layout as a self-contained SVG.

use serde_json::Value;
use std::{env, error::Error, fs, io::Write, path::Path, process::ExitCode};

const MAX_USED_LAYERS: usize = 5;
const KEY_WIDTH: i32 = 52;
const KEY_HEIGHT: i32 = 40;
const KEY_PITCH: i32 = 58;
const SVG_WIDTH: i32 = 1240;
const LAYER_HEIGHT: i32 = 314;
const USAGE: &str = "Render a Cornix Vial .vil file as an SVG keymap diagram.\n\nUsage: render-keymap <input.vil> <output.svg>";

struct LayerMapping {
    left_rows: [[String; 6]; 4],
    right_rows: [[String; 6]; 4],
    left_encoder: [String; 3],
    right_encoder: [String; 3],
}

fn as_keycode(value: &Value) -> String {
    match value {
        Value::String(code) => code.clone(),
        Value::Number(number) if number.as_f64() == Some(-1.0) => "KC_NO".into(),
        _ => value.to_string(),
    }
}

fn is_empty(code: &str) -> bool {
    matches!(code, "KC_NO" | "KC_TRNS")
}

fn layers(vil: &Value) -> Result<&Vec<Value>, String> {
    let layers = vil
        .get("layout")
        .and_then(Value::as_array)
        .ok_or("Vial data must contain a layout array")?;
    for (layer_index, layer) in layers.iter().enumerate() {
        let rows = layer
            .as_array()
            .filter(|rows| rows.len() == 8)
            .ok_or_else(|| format!("Layer {layer_index} must contain 8 Cornix rows"))?;
        for (row_index, row) in rows.iter().enumerate() {
            if row.as_array().is_none_or(|slots| slots.len() < 7) {
                return Err(format!(
                    "Layer {layer_index}, row {row_index} must contain 7 slots"
                ));
            }
        }
    }
    Ok(layers)
}

fn used_layer_indexes(vil: &Value) -> Result<Vec<usize>, String> {
    let indexes: Vec<_> = layers(vil)?
        .iter()
        .enumerate()
        .filter(|(_, layer)| {
            layer.as_array().unwrap().iter().any(|row| {
                row.as_array().unwrap()[..6]
                    .iter()
                    .any(|value| !is_empty(&as_keycode(value)))
            })
        })
        .map(|(index, _)| index)
        .collect();
    if indexes.len() > MAX_USED_LAYERS {
        return Err(format!(
            "The README diagram supports at most {MAX_USED_LAYERS} in-use layers; found {} ({})",
            indexes.len(),
            indexes
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Ok(indexes)
}

fn map_layer(vil: &Value, layer_index: usize) -> Result<LayerMapping, String> {
    let layer = &vil["layout"][layer_index];
    let rotation = |side: usize| -> Option<[String; 2]> {
        let codes = vil
            .get("encoder_layout")?
            .as_array()?
            .get(layer_index)?
            .as_array()?
            .get(side)?
            .as_array()?;
        (codes.len() == 2).then(|| [as_keycode(&codes[0]), as_keycode(&codes[1])])
    };
    let encoder_error = || format!("Layer {layer_index} must define two Cornix encoders");
    let [left_ccw, left_cw] = rotation(0).ok_or_else(encoder_error)?;
    let [right_ccw, right_cw] = rotation(1).ok_or_else(encoder_error)?;
    Ok(LayerMapping {
        left_rows: std::array::from_fn(|row| {
            std::array::from_fn(|column| as_keycode(&layer[row][column]))
        }),
        right_rows: std::array::from_fn(|row| {
            std::array::from_fn(|column| as_keycode(&layer[row + 4][5 - column]))
        }),
        left_encoder: [left_ccw, left_cw, as_keycode(&layer[2][6])],
        right_encoder: [right_ccw, right_cw, as_keycode(&layer[5][6])],
    })
}

fn display_label(keycode: &str) -> String {
    let label = match keycode {
        "KC_BSPACE" => "Bksp",
        "KC_DELETE" => "Del",
        "KC_ENTER" => "Enter",
        "KC_ESCAPE" => "Esc",
        "KC_GESC" => "` / Esc",
        "KC_TAB" => "Tab",
        "KC_SPACE" => "Space",
        "KC_LCTRL" => "L Ctrl",
        "KC_RCTRL" => "R Ctrl",
        "KC_LSHIFT" => "L Shift",
        "KC_RSHIFT" => "R Shift",
        "KC_LALT" => "L Alt",
        "KC_RALT" => "R Alt",
        "KC_LGUI" => "L GUI",
        "KC_RGUI" => "R GUI",
        "KC_UP" => "↑",
        "KC_DOWN" => "↓",
        "KC_LEFT" => "←",
        "KC_RIGHT" => "→",
        "KC_BSLASH" => "\\",
        "KC_SLASH" => "/",
        "KC_DOT" => ".",
        "KC_COMMA" => ",",
        "KC_SCOLON" => ";",
        "KC_MINUS" => "-",
        "KC_EQUAL" => "=",
        "KC_QUOTE" => "'",
        "KC_GRAVE" => "`",
        "KC_LBRACKET" => "[",
        "KC_RBRACKET" => "]",
        "KC_JYEN" => "¥",
        "KC_MUTE" => "Mute",
        "KC_VOLU" => "Vol +",
        "KC_VOLD" => "Vol −",
        "KC_WH_L" => "Wheel ←",
        "KC_WH_R" => "Wheel →",
        "KC_WH_U" => "Wheel ↑",
        "KC_WH_D" => "Wheel ↓",
        "KC_MS_L" => "Mouse ←",
        "KC_MS_R" => "Mouse →",
        "KC_MS_U" => "Mouse ↑",
        "KC_MS_D" => "Mouse ↓",
        "KC_BTN1" => "Mouse 1",
        "KC_BTN2" => "Mouse 2",
        "KC_BTN3" => "Mouse 3",
        "KC_WBAK" => "Back",
        "KC_WFWD" => "Forward",
        "KC_WREF" => "Reload",
        "KC_NO" => "—",
        "KC_TRNS" => "▽",
        _ => {
            if let Some(inner) = keycode
                .strip_prefix("LSFT(")
                .and_then(|value| value.strip_suffix(')'))
                .filter(|value| !value.is_empty() && !value.contains('\n'))
            {
                return format!("⇧ {}", display_label(inner));
            }
            if let Some(index) = keycode
                .strip_prefix("MO(")
                .and_then(|value| value.strip_suffix(')'))
                .filter(|value| {
                    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
                })
            {
                return format!("MO {index}");
            }
            if let Some(index) = keycode.strip_prefix("USER").filter(|value| {
                !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
            }) {
                let trimmed = index.trim_start_matches('0');
                return format!("User {}", if trimmed.is_empty() { "0" } else { trimmed });
            }
            return keycode
                .strip_prefix("KC_")
                .unwrap_or(keycode)
                .replace('_', " ");
        }
    };
    label.into()
}

fn key_kind(keycode: &str) -> &'static str {
    if is_empty(keycode) {
        "empty"
    } else if ["MO(", "TG(", "TO(", "DF(", "LT("]
        .iter()
        .any(|prefix| keycode.starts_with(prefix))
    {
        "layer-key"
    } else if keycode.starts_with("USER") {
        "custom"
    } else if ["CTRL", "SHIFT", "ALT", "GUI", "LSFT(", "RSFT("]
        .iter()
        .any(|token| keycode.contains(token))
    {
        "modifier"
    } else if ["KC_MS_", "KC_WH_", "KC_BTN"]
        .iter()
        .any(|prefix| keycode.starts_with(prefix))
        || matches!(keycode, "KC_WBAK" | "KC_WFWD" | "KC_WREF")
    {
        "pointer"
    } else {
        "normal"
    }
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn render_key(lines: &mut Vec<String>, x: i32, y: i32, keycode: &str, slot: &str, rotation: i32) {
    let center_x = x + KEY_WIDTH / 2;
    let center_y = y + KEY_HEIGHT / 2;
    let transform = if rotation == 0 {
        String::new()
    } else {
        format!(" transform=\"rotate({rotation} {center_x} {center_y})\"")
    };
    let label = display_label(keycode);
    let font_class = if label.chars().count() > 8 {
        " key-label--small"
    } else {
        ""
    };
    lines.extend([
        format!(
            "<g class=\"key {}\" data-slot=\"{}\"{transform}>",
            key_kind(keycode),
            escape(slot)
        ),
        format!("<title>{}</title>", escape(keycode)),
        format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{KEY_WIDTH}\" height=\"{KEY_HEIGHT}\" rx=\"8\"/>"
        ),
        format!(
            "<text class=\"key-label{font_class}\" x=\"{center_x}\" y=\"{}\">{}</text>",
            center_y + 1,
            escape(&label)
        ),
        "</g>".into(),
    ]);
}

fn render_half(lines: &mut Vec<String>, mapping: &LayerMapping, layer_y: i32, is_left: bool) {
    let (rows, start_x, offsets, side) = if is_left {
        (&mapping.left_rows, 34, [12, 6, 0, 3, 8, 14], "left")
    } else {
        (&mapping.right_rows, 854, [14, 8, 3, 0, 6, 12], "right")
    };
    let top_y = layer_y + 48;
    for (row_index, row) in rows[..3].iter().enumerate() {
        for (column, keycode) in row.iter().enumerate() {
            render_key(
                lines,
                start_x + column as i32 * KEY_PITCH,
                top_y + row_index as i32 * 44 + offsets[column],
                keycode,
                &format!("{side}-r{row_index}-c{column}"),
                0,
            );
        }
    }
    if is_left {
        for (column, keycode) in rows[3][..3].iter().enumerate() {
            render_key(
                lines,
                start_x + column as i32 * KEY_PITCH,
                top_y + 132 + offsets[column],
                keycode,
                &format!("left-r3-c{column}"),
                0,
            );
        }
        for (thumb, keycode) in rows[3][3..].iter().enumerate() {
            render_key(
                lines,
                start_x + (thumb as i32 + 3) * KEY_PITCH - 12,
                top_y + 188 + thumb as i32 * 5,
                keycode,
                &format!("left-thumb-{thumb}"),
                8 + thumb as i32 * 2,
            );
        }
    } else {
        // Rows have already been mirrored: thumbs first, outer keys last.
        for (thumb, keycode) in rows[3][..3].iter().enumerate() {
            render_key(
                lines,
                start_x + thumb as i32 * KEY_PITCH + 12,
                top_y + 198 - thumb as i32 * 5,
                keycode,
                &format!("right-thumb-{thumb}"),
                -12 + thumb as i32 * 2,
            );
        }
        for (column, offset) in offsets.iter().enumerate().skip(3) {
            render_key(
                lines,
                start_x + column as i32 * KEY_PITCH,
                top_y + 132 + offset,
                &rows[3][column],
                &format!("right-r3-c{column}"),
                0,
            );
        }
    }
}

fn render_encoder(lines: &mut Vec<String>, x: i32, y: i32, name: &str, mappings: &[String; 3]) {
    let [ccw, cw, press] = mappings;
    lines.extend([
        format!("<g class=\"encoder\" transform=\"translate({x} {y})\">"),
        format!(
            "<title>{}: {}, {}, {}</title>",
            escape(name),
            escape(ccw),
            escape(cw),
            escape(press)
        ),
        "<rect width=\"154\" height=\"116\" rx=\"12\"/>".into(),
        "<circle cx=\"77\" cy=\"31\" r=\"15\"/>".into(),
        format!(
            "<text class=\"encoder-name\" x=\"77\" y=\"62\">{}</text>",
            escape(name)
        ),
        format!(
            "<text class=\"encoder-action\" x=\"77\" y=\"82\">↺ {}</text>",
            escape(&display_label(ccw))
        ),
        format!(
            "<text class=\"encoder-action\" x=\"77\" y=\"98\">{} ↻</text>",
            escape(&display_label(cw))
        ),
        format!(
            "<text class=\"encoder-press\" x=\"77\" y=\"111\">Press: {}</text>",
            escape(&display_label(press))
        ),
        "</g>".into(),
    ]);
}

fn render_keymap(vil: &Value) -> Result<String, String> {
    let layer_indexes = used_layer_indexes(vil)?;
    if layer_indexes.is_empty() {
        return Err("No in-use keymap layers were found".into());
    }
    let height = 76 + layer_indexes.len() as i32 * LAYER_HEIGHT + 18;
    let mut lines = vec![
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>".into(),
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{SVG_WIDTH}\" height=\"{height}\" viewBox=\"0 0 {SVG_WIDTH} {height}\" role=\"img\" aria-labelledby=\"svg-title svg-description\">"
        ),
    ];
    lines.extend([
        "<title id=\"svg-title\">Cornix keymap</title>",
        "<desc id=\"svg-description\">Up to five in-use layers rendered from cornix/main.vil, including both encoders.</desc>",
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
        "<text class=\"page-title\" x=\"24\" y=\"34\">Cornix keymap</text>",
        "<text class=\"page-subtitle\" x=\"24\" y=\"55\">Generated from cornix/main.vil · orange keys switch layers · hover a key for its raw code</text>",
    ].map(str::to_owned));
    for (position, layer_index) in layer_indexes.into_iter().enumerate() {
        let card_y = 72 + position as i32 * LAYER_HEIGHT;
        let mapping = map_layer(vil, layer_index)?;
        lines.extend([
            format!("<g id=\"layer-{layer_index}\">"),
            format!("<rect class=\"layer-card\" x=\"16\" y=\"{card_y}\" width=\"1208\" height=\"298\" rx=\"16\"/>"),
            format!("<text class=\"layer-title\" x=\"34\" y=\"{}\">Layer {layer_index}</text>", card_y + 28),
            format!("<text class=\"layer-note\" x=\"1204\" y=\"{}\" text-anchor=\"end\">48 keys + 2 encoders</text>", card_y + 27),
        ]);
        render_half(&mut lines, &mapping, card_y, true);
        render_half(&mut lines, &mapping, card_y, false);
        render_encoder(
            &mut lines,
            448,
            card_y + 92,
            "Left encoder",
            &mapping.left_encoder,
        );
        render_encoder(
            &mut lines,
            638,
            card_y + 92,
            "Right encoder",
            &mapping.right_encoder,
        );
        lines.push("</g>".into());
    }
    lines.push("</svg>".into());
    Ok(lines.join("\n") + "\n")
}

fn write_svg(input: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    let source = fs::read_to_string(input)
        .map_err(|error| format!("Cannot read {}: {error}", input.display()))?;
    let vil: Value = serde_json::from_str(&source)
        .map_err(|error| format!("Invalid Vial JSON in {}: {error}", input.display()))?;
    let svg = render_keymap(&vil)?;
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut destination = tempfile::Builder::new()
        .prefix(".keymap-")
        .suffix(".tmp")
        .tempfile_in(parent)?;
    destination.write_all(svg.as_bytes())?;
    destination.persist(output)?;
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args.len() != 2 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    match write_svg(Path::new(&args[0]), Path::new(&args[1])) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("render-keymap: {error}");
            ExitCode::FAILURE
        }
    }
}
