# jp-address-search

[![Crates.io](https://img.shields.io/crates/v/jp-address-search.svg)](https://crates.io/crates/jp-address-search)
[![docs.rs](https://img.shields.io/docsrs/jp-address-search)](https://docs.rs/jp-address-search)
[![CI](https://github.com/uiuifree/rust-jp-address-search/actions/workflows/ci.yml/badge.svg)](https://github.com/uiuifree/rust-jp-address-search/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

日本の住所検索(郵便番号検索・住所逆引き)を行うRustライブラリです。
郵便番号(zipcode)から住所への変換、都道府県・市区町村マスタの検索、
市区町村の代表点(本庁舎の緯度経度)の取得が、外部API・データベースなしで
オフラインで完結します。

- **自己完結**: 日本郵便(KEN_ALL)と総務省の公式データをバイナリに埋め込み。
  実行時のファイル読み込み・ネットワークアクセス不要、ランタイム依存クレートもゼロ
- **高速**: 郵便番号検索 約6ns、市区町村コード検索は二分探索。検索時のヒープ確保なし
- **正確**: 政令指定都市の行政区、2024年の浜松市区再編などの市区町村変更に追従

**English**: Fast, zero-dependency Japanese address lookup for Rust.
Search addresses by postal code (〒 zipcode → address), look up all 47 prefectures
and 1,900+ municipalities (cities / wards / towns) with their representative
coordinates (city-hall latitude/longitude), with official data from
Japan Post (KEN_ALL), the Ministry of Internal Affairs, and MLIT embedded at
compile time. No runtime I/O, no external API. All searches are exposed through the
single `AddressSearch` entry point — see [docs.rs](https://docs.rs/jp-address-search).

## ユースケース

- 郵便番号から住所の自動入力(郵便番号 → 都道府県・市区町村・町域の変換)
- 住所文字列の解析・正規化(住所 → 都道府県・市区町村・郵便番号の逆引き)
- 都道府県・市区町村マスタ(JISコード、政令指定都市の行政区)の参照
- 市区町村コードから緯度経度への変換(周辺検索・地図表示の起点、ジオコーディング)

## インストール

```toml
[dependencies]
jp-address-search = "0.3"
```

## 使い方

検索はすべて `AddressSearch` に集約されています。

### 郵便番号から住所を検索

```rust
use jp_address_search::AddressSearch;

// ハイフンの有無は問わない("1000001" でも "100-0001" でも同じ結果)
let addresses = AddressSearch::find_by_zipcode("100-0001");
let address = addresses[0];
assert_eq!(address.prefecture_name, "東京都");
assert_eq!(address.city_name, "千代田区");
assert_eq!(address.town, "千代田");

// 1つの郵便番号が複数の町域に対応する場合があるためスライスが返る
let addresses = AddressSearch::find_by_zipcode("069-1329");
assert_eq!(addresses.len(), 3);
```

### 住所から都道府県・市区町村・郵便番号を特定

```rust
use jp_address_search::AddressSearch;

let matched = AddressSearch::parse_address("東京都世田谷区太子堂4丁目1-1");
assert_eq!(matched.prefecture.unwrap().name, "東京都");
assert_eq!(matched.city.unwrap().name, "世田谷区");
assert_eq!(matched.address.unwrap().zip, "1540004");

// 都道府県の省略、郡名の有無、ヶ/ケ/が・全角数字・空白の表記ゆれに対応
let matched = AddressSearch::parse_address("千代田区霞ヶ関1丁目");
assert_eq!(matched.address.unwrap().zip, "1000013"); // データ上は「霞が関」

// 丁目で郵便番号が分かれる町域(札幌市中央区大通西など)は丁目で特定
let matched = AddressSearch::parse_address("札幌市中央区大通西25丁目");
assert_eq!(matched.address.unwrap().zip, "0640820"); // 20〜28丁目
```

※ 解析は町域+丁目まで。番地は解析対象外です。

### 都道府県

```rust
use jp_address_search::AddressSearch;

let prefecture = AddressSearch::find_prefecture_by_id(13).unwrap();
assert_eq!(prefecture.name, "東京都");
assert_eq!(prefecture.short_name, "東京");
assert_eq!(prefecture.en_name, "tokyo");

let prefecture = AddressSearch::find_prefecture_by_name("大阪府").unwrap();
assert_eq!(prefecture.id, 27);

// 全47都道府県
assert_eq!(AddressSearch::prefectures().len(), 47);
```

### 市区町村

```rust
use jp_address_search::AddressSearch;

let city = AddressSearch::find_city_by_id(27141).unwrap();
assert_eq!(city.name, "堺市堺区");
assert_eq!(city.prefecture_id, 27);
// 政令指定都市の行政区は親となる市のコードを持つ
assert_eq!(city.designated_city_id, 27140);

// 都道府県内の市区町村一覧
let cities = AddressSearch::find_cities_by_prefecture_id(13);
assert!(!cities.is_empty());

// 同名の市区町村(府中市=東京都・広島県など)はコード最小のものが返る
let fuchu = AddressSearch::find_city_by_name("府中市").unwrap();
assert_eq!(fuchu.id, 13206); // 東京都府中市
```

### 市区町村コードから緯度経度(代表点)を取得

市区町村コードから代表点(本庁舎の位置)の緯度経度を引けます。
周辺検索・距離計算の起点や地図表示に使えます。

```rust
use jp_address_search::AddressSearch;

// 渋谷区 → 渋谷区役所の位置
let location = AddressSearch::find_city_location(13113).unwrap();
assert!((location.latitude - 35.664).abs() < 0.01);
assert!((location.longitude - 139.698).abs() < 0.01);

// 政令指定都市は本体(市役所)と行政区(区役所)の両方を持つ
assert!(AddressSearch::find_city_location(27100).is_some()); // 大阪市
assert!(AddressSearch::find_city_location(27127).is_some()); // 大阪市北区
```

- 役場が自治体の外にある場合(竹富町役場は石垣市など)も役場の実際の所在地を
  返します
- 役場が実在しない北方領土の6村(色丹村・泊村・留夜別村・留別村・紗那村・
  蘂取村)は `None` を返します

### 初回ロードの事前実行

データは初回アクセス時に構築されます(数ms)。初回検索の遅延を避けたい場合は、
アプリ起動時に `preload()` を呼んでください。

```rust
jp_address_search::AddressSearch::preload();
```

## ベンチマーク

releaseビルドでの実測値(13th Gen Intel Core i9-13900K / WSL2 / rustc 1.97)。
各値は**100万回実行の平均**(`parse_address`のみ10万回実行の平均)です。
検索はいずれもヒープ確保なしで、`'static`スライス・参照を返します。

| API | 速度 |
|---|---|
| `find_by_zipcode`(郵便番号→住所) | **7ns** |
| `find_prefecture_by_id` | 5ns |
| `find_city_by_id`(二分探索) | 7ns |
| `find_cities_by_prefecture_id` | 23ns |
| `find_addresses_by_city_id` | 43ns |
| `parse_address`(住所文字列の解析、都道府県あり) | 2.2µs |
| `parse_address`(同上、都道府県省略) | 27µs |
| 初回ロード(`preload`、プロセスで1回) | 11ms |

再現方法:

```sh
cargo run --release --example bench
```

## データソース

| データ | 出典 | 同梱データの版 |
|---|---|---|
| 都道府県・市区町村 | 総務省「[全国地方公共団体コード](https://www.soumu.go.jp/denshijiti/code.html)」 | 令和6年1月1日現在 |
| 郵便番号 | 日本郵便「[住所の郵便番号(1レコード1行、UTF-8形式)](https://www.post.japanpost.jp/service/search/zipcode/download/utf-zip.html)」 | 2026年6月版 |
| 市区町村の代表点 | 国土数値情報「[市区町村役場等及び公的集会施設データ](https://nlftp.mlit.go.jp/ksj/gml/datalist/KsjTmplt-P05-v3_0.html)」(国土交通省) | 令和4年度版 |

市区町村の代表点は「国土数値情報(国土交通省)」を加工して作成しています
(2024年1月の浜松市行政区再編の読み替えなど。加工内容は
`src/bin/update_city_location.rs` を参照)。

## データの更新

埋め込みデータ(`src/data/*.tsv`)は`generate` featureのバイナリで再生成できます。
`generate` featureにはRust 1.88以上が必要です(ライブラリ本体は1.80以上で動きます)。

```sh
# 市区町村マスタ: 「都道府県コード及び市区町村コード」のExcelを
# storage/code.xlsx として配置して実行
cargo run --features generate --bin update_master

# 郵便番号データ: utf_ken_all.zip を展開した storage/utf_ken_all.csv を配置して実行
cargo run --features generate --bin update_address

# 市区町村の代表点: 国土数値情報P05の都道府県別zipを展開したGeoJSONを
# storage/p05/P05-22_01.geojson 〜 P05-22_47.geojson として配置して実行
cargo run --features generate --bin update_city_location
```

## ドキュメント

- APIリファレンス: [docs.rs/jp-address-search](https://docs.rs/jp-address-search)
- AIエージェント・LLM向けのAPI要約: [llms.txt](llms.txt)
- 変更履歴: [CHANGELOG.md](CHANGELOG.md)

## ライセンス

[MIT](LICENSE)
