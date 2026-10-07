# My Cornix key mapping

![cornix/main.vil のキーマッピング](docs/cornix-keymap.svg)

## for update

Vialで保存した `cornix/main.vil` をコミットして `master` へpushすると、[GitHub Actions](.github/workflows/render-keymap.yml)がRustで図を生成し、変更があれば `docs/cornix-keymap.svg` をbotが自動コミットします。READMEの図も更新されます。ローカルでのスクリプト実行やコミットフックの設定は不要です。

PRではテストと図の生成を行い、生成したSVGをActionsの成果物 `cornix-keymap` に保存します。SVGを手元で更新してコミットする必要はありません。マージ後に `master` の図が自動更新されます。Actionsの「Run workflow」から再生成することもできます。

ローカルで確認する場合はRust/Cargoを使います。

```bash
cargo test --locked
cargo run --locked -- cornix/main.vil docs/cornix-keymap.svg
```
