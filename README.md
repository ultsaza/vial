# Cornix Vial keymap

`cornix/main.vil` を正として、Cornix LP の物理配置に合わせたキーマップ図を自動生成しています。

![cornix/main.vil のキーマッピング](docs/cornix-keymap.svg)

## 更新方法

Vial でレイアウトを保存して `cornix/main.vil` を更新し、`master` へ push すると、[Render Cornix keymap](.github/workflows/render-keymap.yml) がテストと再描画を行います。図に差分がある場合は `github-actions[bot]` が `docs/cornix-keymap.svg` だけをコミットします。

ローカルで確認・再生成する場合:

```bash
python3 -m unittest discover -s tests -v
python3 scripts/render_keymap.py cornix/main.vil docs/cornix-keymap.svg
```

Vial が末尾に書き出す未使用レイヤーは図から除外します。通常キーが割り当てられたレイヤーは最大5つまで対応し、6つ以上になった場合は図を黙って欠落させず、生成処理をエラーにします。

## 参考にしたツール・図

- [keymap-drawer](https://github.com/caksoylar/keymap-drawer) — プログラムから SVG を生成する構成と自動更新ワークフロー
- [Vial layout to Keymap Drawer converter](https://github.com/YAL-Tools/vial-to-keymap-drawer) — `.vil` 変換時のキー順序・未使用スロットに関する注意点
- [CornixShow](https://iorinu.github.io/cornix-show/) — Cornix LP の左右分離、カラムスタッガード、親指クラスタ、ノブ配置の参考図

このリポジトリでは、機種依存のキー順序を明示的に扱い、外部ランタイムなしで再生成できるように、標準ライブラリだけの専用レンダラーを使用しています。
