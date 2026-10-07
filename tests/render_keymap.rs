use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(input: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_render-keymap"))
        .arg(input)
        .arg(output)
        .output()
        .unwrap()
}

fn placeholder_layer() -> Value {
    let mut rows = vec![vec![json!("KC_NO"); 7]; 8];
    for row in &mut rows {
        row[6] = json!(-1);
    }
    rows[2][6] = json!("KC_MUTE");
    rows[5][6] = json!("KC_BTN3");
    json!(rows)
}

fn vial(layers: Vec<Value>) -> Value {
    let encoders = vec![json!([["KC_WH_L", "KC_WH_R"], ["KC_WH_U", "KC_WH_D"]]); layers.len()];
    json!({"layout": layers, "encoder_layout": encoders})
}

fn base_vial() -> Value {
    let mut layer = placeholder_layer();
    layer[0][0] = json!("KC_A");
    vial(vec![layer])
}

fn render(data: Value) -> String {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.vil");
    let output = directory.path().join("keymap.svg");
    fs::write(&input, data.to_string()).unwrap();
    let result = run(&input, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stderr.is_empty());
    assert!(output.exists(), "renderer did not create the SVG");
    fs::read_to_string(output).unwrap()
}

fn key<'a>(svg: &'a str, slot: &str) -> &'a str {
    svg.split_once(&format!("data-slot=\"{slot}\""))
        .unwrap()
        .1
        .split_once("</g>")
        .unwrap()
        .0
}

#[test]
fn preserves_the_python_renderers_svg_for_a_reference_layer() {
    let data = serde_json::from_str(include_str!("fixtures/single-layer.vil")).unwrap();
    assert_eq!(render(data), include_str!("fixtures/single-layer.svg"));
}

#[test]
fn ignores_unused_layers_even_with_default_encoders() {
    let mut transparent = placeholder_layer();
    for row in transparent.as_array_mut().unwrap() {
        for slot in &mut row.as_array_mut().unwrap()[..6] {
            *slot = json!("KC_TRNS");
        }
    }
    let mut navigation = placeholder_layer();
    navigation[4][2] = json!("KC_LEFT");
    let svg = render(vial(vec![
        base_vial()["layout"][0].clone(),
        transparent,
        navigation,
        placeholder_layer(),
    ]));
    assert!(svg.contains("id=\"layer-0\""));
    assert!(svg.contains("id=\"layer-2\""));
    assert!(!svg.contains("id=\"layer-1\""));
    assert!(!svg.contains("id=\"layer-3\""));
    assert_eq!(svg.matches("class=\"key ").count(), 96);
    assert_eq!(svg.matches("class=\"encoder\"").count(), 4);
}

#[test]
fn supports_five_used_layers() {
    let svg = render(vial(vec![base_vial()["layout"][0].clone(); 5]));
    assert_eq!(svg.matches("class=\"key ").count(), 240);
    assert_eq!(svg.matches("class=\"encoder\"").count(), 10);
    assert!(svg.contains("height=\"1664\""));
}

#[test]
fn maps_mirrored_right_hand_keys_thumbs_and_encoder_presses() {
    let mut layer = placeholder_layer();
    for row in 0..8 {
        for column in 0..6 {
            layer[row][column] = json!(format!("R{row}C{column}"));
        }
    }
    layer[2][6] = json!("LEFT_PRESS");
    layer[5][6] = json!("RIGHT_PRESS");
    let mut data = vial(vec![layer]);
    data["encoder_layout"][0] = json!([["LEFT_CCW", "LEFT_CW"], ["RIGHT_CCW", "RIGHT_CW"]]);
    let svg = render(data);
    assert!(key(&svg, "left-r0-c0").contains("<title>R0C0</title>"));
    assert!(key(&svg, "right-r0-c0").contains("<title>R4C5</title>"));
    assert!(key(&svg, "right-r0-c5").contains("<title>R4C0</title>"));
    assert!(key(&svg, "left-thumb-0").contains("<title>R3C3</title>"));
    assert!(key(&svg, "right-thumb-0").contains("<title>R7C5</title>"));
    assert!(key(&svg, "right-r3-c3").contains("<title>R7C2</title>"));
    assert!(svg.contains("<title>Left encoder: LEFT_CCW, LEFT_CW, LEFT_PRESS</title>"));
    assert!(svg.contains("<title>Right encoder: RIGHT_CCW, RIGHT_CW, RIGHT_PRESS</title>"));
}

#[test]
fn preserves_compact_labels_and_key_categories() {
    for (code, label, kind) in [
        ("KC_W", "W", "normal"),
        ("KC_WBAK", "Back", "pointer"),
        ("KC_WFWD", "Forward", "pointer"),
        ("KC_WREF", "Reload", "pointer"),
        ("LSFT(KC_1)", "⇧ 1", "modifier"),
        ("LSFT(LSFT(KC_A))", "⇧ ⇧ A", "modifier"),
        ("MO(4)", "MO 4", "layer-key"),
        ("USER02", "User 2", "custom"),
        ("KC_LCTRL", "L Ctrl", "modifier"),
        ("KC_MS_U", "Mouse ↑", "pointer"),
        ("KC_F12", "F12", "normal"),
        ("KC_SOMETHING_NEW", "SOMETHING NEW", "normal"),
    ] {
        let mut data = base_vial();
        data["layout"][0][0][0] = json!(code);
        let svg = render(data);
        assert!(
            svg.contains(&format!("class=\"key {kind}\" data-slot=\"left-r0-c0\"")),
            "{code}"
        );
        assert!(
            key(&svg, "left-r0-c0").contains(&format!(">{label}</text>")),
            "{code}"
        );
    }
}

#[test]
fn escapes_raw_codes_and_labels_as_xml() {
    let mut data = base_vial();
    data["layout"][0][0][0] = json!("USER<&\"'");
    data["encoder_layout"][0][0][0] = json!("KC_<&\"'");
    let svg = render(data);
    assert!(key(&svg, "left-r0-c0").contains("<title>USER&lt;&amp;&quot;&#x27;</title>"));
    assert!(svg.contains("↺ &lt;&amp;&quot;&#x27;</text>"));
    assert!(svg.ends_with('\n'));
}

#[test]
fn treats_numeric_minus_one_as_an_empty_key() {
    let mut data = base_vial();
    data["layout"][0][0][1] = json!(-1);
    let svg = render(data);
    assert!(key(&svg, "left-r0-c1").contains("<title>KC_NO</title>"));
    assert!(key(&svg, "left-r0-c1").contains(">—</text>"));
    assert!(svg.contains("class=\"key empty\" data-slot=\"left-r0-c1\""));
}

#[test]
fn measures_unicode_label_length_in_characters() {
    let mut data = base_vial();
    data["layout"][0][0][0] = json!("KC_矢印←→↑↓");
    let svg = render(data.clone());
    assert!(key(&svg, "left-r0-c0").contains("class=\"key-label\""));
    data["layout"][0][0][0] = json!("KC_ABCDEFGHI");
    assert!(key(&render(data), "left-r0-c0").contains("key-label--small"));
}

#[test]
fn rejects_invalid_layouts_without_overwriting_the_output() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.vil");
    let output = directory.path().join("keymap.svg");
    let mut short_rows = base_vial();
    short_rows["layout"][0].as_array_mut().unwrap().pop();
    let mut short_slots = base_vial();
    short_slots["layout"][0][0].as_array_mut().unwrap().pop();
    let mut missing_encoders = base_vial();
    missing_encoders
        .as_object_mut()
        .unwrap()
        .remove("encoder_layout");
    let mut short_rotation = base_vial();
    short_rotation["encoder_layout"][0][1] = json!(["KC_A"]);
    let mut malformed_row = base_vial();
    malformed_row["layout"][0][1] = Value::Null;
    let mut malformed_encoder = base_vial();
    malformed_encoder["encoder_layout"][0][0] = json!("AB");
    for (data, error) in [
        (json!({}), "Vial data must contain a layout array"),
        (
            json!({"layout": null}),
            "Vial data must contain a layout array",
        ),
        (short_rows, "Layer 0 must contain 8 Cornix rows"),
        (short_slots, "Layer 0, row 0 must contain 7 slots"),
        (malformed_row, "Layer 0, row 1 must contain 7 slots"),
        (missing_encoders, "Layer 0 must define two Cornix encoders"),
        (short_rotation, "Layer 0 must define two Cornix encoders"),
        (malformed_encoder, "Layer 0 must define two Cornix encoders"),
        (
            vial(vec![placeholder_layer()]),
            "No in-use keymap layers were found",
        ),
        (
            vial(vec![base_vial()["layout"][0].clone(); 6]),
            "at most 5 in-use layers",
        ),
    ] {
        fs::write(&input, data.to_string()).unwrap();
        fs::write(&output, "previous diagram").unwrap();
        let result = run(&input, &output);
        assert!(!result.status.success(), "accepted {data}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(error),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(fs::read_to_string(&output).unwrap(), "previous diagram");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}

#[test]
fn reports_missing_and_invalid_json_input_without_overwriting_output() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.vil");
    let output = directory.path().join("keymap.svg");
    fs::write(&output, "previous diagram").unwrap();
    assert!(!run(&input, &output).status.success());
    fs::write(&input, "{ invalid JSON").unwrap();
    assert!(!run(&input, &output).status.success());
    assert_eq!(fs::read_to_string(output).unwrap(), "previous diagram");
}

#[test]
fn creates_output_directories_and_replaces_an_existing_diagram() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.vil");
    let output = directory.path().join("nested/docs/keymap.svg");
    fs::write(&input, base_vial().to_string()).unwrap();
    assert!(run(&input, &output).status.success());
    let first = fs::read_to_string(&output).unwrap();
    let mut data = base_vial();
    data["layout"][0][0][0] = json!("KC_B");
    fs::write(&input, data.to_string()).unwrap();
    assert!(run(&input, &output).status.success());
    let second = fs::read_to_string(&output).unwrap();
    assert_ne!(first, second);
    assert!(key(&second, "left-r0-c0").contains("<title>KC_B</title>"));
    assert_eq!(fs::read_dir(output.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn supports_a_bare_relative_output_filename() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("input.vil"), base_vial().to_string()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_render-keymap"))
        .current_dir(directory.path())
        .args(["input.vil", "keymap.svg"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(directory.path().join("keymap.svg").is_file());
}

#[test]
fn cleans_up_a_temporary_file_when_output_replacement_fails() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.vil");
    let output = directory.path().join("keymap.svg");
    fs::write(&input, base_vial().to_string()).unwrap();
    fs::create_dir(&output).unwrap();
    assert!(!run(&input, &output).status.success());
    assert!(output.is_dir());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn explains_cli_usage_and_rejects_wrong_argument_counts() {
    for args in [
        vec![],
        vec!["input.vil"],
        vec!["input.vil", "output.svg", "extra"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_render-keymap"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&result.stderr).contains("Usage:"));
    }
    let result = Command::new(env!("CARGO_BIN_EXE_render-keymap"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("Usage:"));
}
