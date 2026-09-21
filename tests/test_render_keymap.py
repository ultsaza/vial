import unittest

from scripts.render_keymap import (
    display_label,
    map_layer,
    render_keymap,
    used_layer_indexes,
)


def placeholder_layer() -> list[list[object]]:
    rows = [["KC_NO"] * 6 + [-1] for _ in range(8)]
    rows[2][6] = "KC_MUTE"
    rows[5][6] = "KC_BTN3"
    return rows


def vial_with_layers(layers: list[list[list[object]]]) -> dict[str, object]:
    return {
        "layout": layers,
        "encoder_layout": [
            [["KC_WH_L", "KC_WH_R"], ["KC_WH_U", "KC_WH_D"]]
            for _ in layers
        ],
    }


class UsedLayerTests(unittest.TestCase):
    def test_ignores_vial_placeholder_layers_with_encoder_defaults(self) -> None:
        base = placeholder_layer()
        base[0][0] = "KC_A"
        navigation = placeholder_layer()
        navigation[4][2] = "KC_LEFT"
        vil = vial_with_layers(
            [base, navigation, placeholder_layer(), placeholder_layer()]
        )

        self.assertEqual(used_layer_indexes(vil), [0, 1])

    def test_rejects_a_sixth_used_layer(self) -> None:
        layers = []
        for index in range(6):
            layer = placeholder_layer()
            layer[0][0] = f"USER{index:02d}"
            layers.append(layer)

        with self.assertRaisesRegex(ValueError, "at most 5"):
            used_layer_indexes(vial_with_layers(layers))


class CornixMappingTests(unittest.TestCase):
    def test_maps_right_half_in_physical_left_to_right_order(self) -> None:
        layer = [
            [f"L{row}{column}" for column in range(6)] + [-1]
            for row in range(4)
        ] + [
            [f"R{row}{column}" for column in range(6)] + [-1]
            for row in range(4)
        ]
        layer[2][6] = "LEFT_PRESS"
        layer[5][6] = "RIGHT_PRESS"
        vil = {
            "layout": [layer],
            "encoder_layout": [
                [["LEFT_CCW", "LEFT_CW"], ["RIGHT_CCW", "RIGHT_CW"]]
            ],
        }

        mapping = map_layer(vil, 0)

        self.assertEqual(mapping.left_rows[0], tuple(f"L0{i}" for i in range(6)))
        self.assertEqual(
            mapping.right_rows[0], tuple(f"R0{i}" for i in reversed(range(6)))
        )
        self.assertEqual(
            mapping.left_encoder, ("LEFT_CCW", "LEFT_CW", "LEFT_PRESS")
        )
        self.assertEqual(
            mapping.right_encoder, ("RIGHT_CCW", "RIGHT_CW", "RIGHT_PRESS")
        )


class SvgRenderingTests(unittest.TestCase):
    def test_plain_w_key_is_not_styled_as_a_pointer_key(self) -> None:
        base = placeholder_layer()
        base[0][0] = "KC_W"

        svg = render_keymap(vial_with_layers([base]))

        self.assertIn(
            '<g class="key normal" data-slot="left-r0-c0">\n<title>KC_W</title>',
            svg,
        )

    def test_uses_compact_browser_labels_that_fit_a_key(self) -> None:
        self.assertEqual(display_label("KC_WBAK"), "Back")
        self.assertEqual(display_label("KC_WFWD"), "Forward")

    def test_renders_only_used_layers_and_preserves_raw_codes_in_titles(self) -> None:
        base = placeholder_layer()
        base[0][0] = "USER<&"
        mouse = placeholder_layer()
        mouse[6][3] = "KC_MS_U"
        vil = vial_with_layers([base, mouse, placeholder_layer()])

        svg = render_keymap(vil)

        self.assertIn('id="layer-0"', svg)
        self.assertIn('id="layer-1"', svg)
        self.assertNotIn('id="layer-2"', svg)
        self.assertIn("<title>USER&lt;&amp;</title>", svg)
        self.assertIn("Left encoder", svg)
        self.assertIn("Right encoder", svg)
        self.assertTrue(svg.endswith("\n"))


if __name__ == "__main__":
    unittest.main()
