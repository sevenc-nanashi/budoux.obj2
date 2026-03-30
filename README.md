<div style="text-align: center;">
    <img width="100%" src="./thumbnail.webp">
</div>

# budoux.obj2

[![AviUtl2 Catalog](https://aviutl2-catalog-badge.sevenc7c.workers.dev/badge/v/sevenc-nanashi.budoux-obj2)](https://aviutl2-catalog-badge.sevenc7c.workers.dev/package/sevenc-nanashi.budoux-obj2)

横幅を指定してテキストを折り返すAviUtl2用スクリプト。

## インストール

[Releases](https://github.com/sevenc-nanashi/budoux.obj2/releases/latest) から `sevenc-nanashi.budoux-obj2-v{{version}}.au2pkg.zip` をダウンロードし、AviUtl2 のプレビューにドラッグ＆ドロップしてください。

## PI

スクリプトはPI（Parameter Injection）を使用することで各種パラメーターをLuaの数式で指定できます。\
PIによって設定された値はトラックバーによる指定より優先されます。

基本的には使う必要はありませんが、PIを使うことでより柔軟な設定が可能になります。

### キー一覧

| キー              | 型      | 説明               |
| ----------------- | ------- | ------------------ |
| `width`           | number  | 横幅               |
| `justify`         | number  | 両端揃え（0〜2）   |
| `align`           | number  | 揃え（0〜11）      |
| `char_spacing`    | number  | 字間               |
| `line_spacing`    | number  | 行間               |
| `speed`           | number  | 表示速度           |
| `size`            | number  | フォントサイズ     |
| `font`            | string  | フォント名         |
| `color`           | number  | 文字色             |
| `secondary_color` | number  | 影・縁色           |
| `decoration`      | number  | 装飾タイプ（0〜6） |
| `bold`            | boolean | 太字               |
| `italic`          | boolean | 斜体               |
| `text`            | string  | テキスト           |
| `debug`           | boolean | デバッグモード     |

## ライセンス

MIT License で公開しています。詳細は [LICENSE](LICENSE) を参照してください。

## 謝辞

このスクリプトは [budoux](https://github.com/google/budoux) をベースにしています。
