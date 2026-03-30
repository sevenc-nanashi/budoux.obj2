<div style="text-align: center;">
    <img width="100%" src="./thumbnail.webp">
</div>

# budoux.obj2

[![AviUtl2 Catalog](https://aviutl2-catalog-badge.sevenc7c.workers.dev/badge/v/sevenc-nanashi.budoux-obj2)](https://aviutl2-catalog-badge.sevenc7c.workers.dev/package/sevenc-nanashi.budoux-obj2)

BudouXという機械学習モデルに基づき、文章を自動的に改行するAviUtl2のスクリプト。

## Tips

- 文章がどのように区切られるかは[BudouXの公式デモ](https://google.github.io/budoux/)を使うと便利です。
  「Replace ZWSP with BR」をオンにすると、どこで改行される可能性があるかがわかります。
- 行末のスペースは削除されます。\
  例：`AviUtl 2.0`と書かれていて、`AviUtl`で行が埋まった場合、`AviUtl<改行>2.0`と改行されます。
- `\b`と書くと文節の区切りとして認識されます。\
  例：`AviUtl\b2.0`の場合、`AviUtl` と `2.0` の間で単語が区切られたとして認識されます。ただし、`AviUtl`で行を使い果たさなかった場合は、`AviUtl2.0`と表示されます。

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
