# AGENTS.md

AIエージェント・コントリビューター向けの開発ガイド。

## プロジェクト概要

日本の住所検索Rustライブラリ。郵便番号・都道府県・市区町村のデータを
TSVでバイナリに埋め込み、`AddressSearch` に集約したAPIで検索する。

- ランタイム依存クレートゼロを維持すること(依存追加は原則不可)
- 公開APIは `AddressSearch` に集約する(データ型に検索メソッドを生やさない)
- 検索結果は `'static` 参照で返し、検索時のヒープ確保を避ける

## コマンド

```sh
cargo test --all-features                            # テスト(unit + doctest)
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps       # ドキュメント検証
```

## 構成

- `src/lib.rs` — `AddressSearch`(全検索APIのファサード)
- `src/prefecture.rs` / `src/city.rs` / `src/city_location.rs` / `src/address.rs` — データ型・埋め込みデータのパース・検索の実装
- `src/data/*.tsv` — 埋め込みデータ(手編集禁止。下記の生成バイナリで再生成)
- `src/bin/update_master.rs` — `storage/code.xlsx`(総務省)→ `cities.tsv`
- `src/bin/update_address.rs` — `storage/utf_ken_all.csv`(日本郵便)→ `zipcode.tsv`
- `src/bin/update_city_location.rs` — `storage/p05/*.geojson`(国土数値情報P05)→ `city_location.tsv`

## 不変条件(テストで担保)

- `zipcode.tsv` は市区町村コード順(`find_addresses_by_city_id` の二分探索が前提)
- `cities.tsv` は市区町村コード順で重複なし(二分探索・都道府県範囲検索が前提)
- `city_location.tsv` は市区町村コード順で重複なし(二分探索が前提)。
  件数は `cities.tsv` から役場が実在しない北方領土の6村を除いた数。
  座標は日本の範囲(緯度20〜46 / 経度122〜154)に収まる
- 同名の市区町村が複数存在する(府中市・伊達市など26種)ため、名称検索はコード最小の1件を返す
- 郵便番号は数字7桁ちょうどで検索(それ以外は空を返す)
- データ更新時はテストの件数フィクスチャ(全件数・都道府県別件数)を実データに合わせて更新する

## データ更新手順

1. 日本郵便: https://www.post.japanpost.jp/service/search/zipcode/download/utf-zip.html
   から `utf_ken_all.zip` を取得し `storage/utf_ken_all.csv` に展開
2. 総務省: https://www.soumu.go.jp/denshijiti/code.html の
   「都道府県コード及び市区町村コード」Excelを `storage/code.xlsx` に配置
3. 国土数値情報P05: https://nlftp.mlit.go.jp/ksj/gml/datalist/KsjTmplt-P05-v3_0.html
   の都道府県別zip(47件。直接URLは
   `https://nlftp.mlit.go.jp/ksj/gml/data/P05/P05-22/P05-22_01_GML.zip` 形式)を
   取得し、GeoJSONを `storage/p05/P05-22_01.geojson` 〜 `P05-22_47.geojson` に展開
4. `cargo run --features generate --bin update_master`
   / `cargo run --features generate --bin update_address`
   / `cargo run --features generate --bin update_city_location` を実行
5. `git diff src/data/` で変更内容を確認し、テストのフィクスチャを更新

`update_city_location` は cities.tsv を正として P05 と突合し、欠け・余り・曖昧を
すべて列挙してエラー終了する。P05 の年度が古く市区町村の変更(合併・政令指定都市の
区再編)に追い付いていない場合や、本庁舎レコードの欠落・誤りがある場合は、
`src/bin/update_city_location.rs` の補正テーブル(`MANUAL_LOCATIONS` / `OVERRIDES` /
`OBSOLETE_CODES`)を実データを確認して更新する。
