# Changelog

## 0.3.1

### 変更

- データ生成用の `generate` feature で使う calamine を 0.21 から 0.36 に更新。
  間接依存の quick-xml が 0.41 になり、RUSTSEC-2026-0194 / RUSTSEC-2026-0195
  の影響を受けなくなった
- `generate` feature には Rust 1.88 以上が必要になった。ライブラリ本体は
  引き続き Rust 1.80 以上で動く(ランタイム依存ゼロのため利用者への影響はない)
- `update_master` はシートを読み込めないとき、そのシートを読み飛ばさずに
  エラー終了するようにした

## 0.3.0

### 追加

- `AddressSearch::find_city_location(city_id)` — 市区町村コードから代表点
  (本庁舎の緯度経度)を返す。データは国土数値情報「市区町村役場等及び
  公的集会施設データ」(国土交通省、令和4年度版)由来で、2024年1月の
  浜松市行政区再編(中央区・浜名区・天竜区)に対応済み
  - 政令指定都市は本体(市役所)と行政区(区役所)の両方を持つ
  - 役場が自治体の外にある場合(竹富町役場は石垣市など)は役場の実際の
    所在地を返す
  - 役場が実在しない北方領土の6村は `None` を返す
- `CityLocation` 型(`city_id` / `latitude` / `longitude`)
- 生成バイナリ `update_city_location`(`--features generate`)。
  cities.tsv と P05 を突合し、欠け・余り・曖昧があればすべて列挙して
  エラー終了する

### 変更

- crates.io メタデータ(description / keywords / categories)と README /
  llms.txt を検索・AIエージェントから見つけやすいように整備

既存のAPI・データ型に変更はありません。

## 0.2.0

- 住所検索ライブラリの全面刷新とデータ更新
