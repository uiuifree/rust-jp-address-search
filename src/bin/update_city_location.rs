//! 国土数値情報「市区町村役場等及び公的集会施設データ(P05)」のGeoJSONから
//! `src/data/city_location.tsv`(市区町村コード → 本庁舎の緯度経度)を生成する。
//!
//! 入力: `./storage/p05/P05-22_01.geojson` 〜 `P05-22_47.geojson`
//! (https://nlftp.mlit.go.jp/ksj/gml/datalist/KsjTmplt-P05-v3_0.html の
//! 都道府県別zipを展開したもの)と `src/data/cities.tsv`
//! 実行: `cargo run --features generate --bin update_city_location`
//!
//! cities.tsvを正として全市区町村に座標を割り当てる。P05は施設の「所在地」の
//! 行政区域コードしか持たないため(村外役場・政令指定都市の市役所は自分のコードに
//! 紐付かない)、施設名称との突合で割り当てる。欠け・余り・曖昧はすべて列挙して
//! エラー終了する。

use csv::WriterBuilder;
use std::collections::{BTreeMap, HashSet};
use std::process::exit;

/// 役場が実在しないため座標を出力しない市区町村(北方領土の6村)
const NO_OFFICE: [i32; 6] = [1695, 1696, 1697, 1698, 1699, 1700];

/// P05に使える本庁舎レコードが無いため座標を直接与える市区町村。
/// 値は国土地理院の住所検索(https://msearch.gsi.go.jp/address-search/AddressSearch)
/// で役場所在地の住所から引いた代表点(大字・字レベル)
const MANUAL_LOCATIONS: [(i32, f64, f64, &str); 3] = [
    // P05に本庁舎(天王字棒沼台226-1)のレコードが無い(出張所・集会施設のみ)
    (5211, 39.862446, 140.007538, "潟上市"),
    // P05の浪江町役場は避難先(二本松市)の座標のまま。2017年帰還後の所在地で補正
    (7547, 37.494923, 141.000259, "浪江町"),
    // P05の飯舘村役場は避難先(福島市)の座標のまま。2016年帰還後の所在地で補正
    (7564, 37.682148, 140.731827, "飯舘村"),
];

/// 名称の機械的な突合では本庁舎を特定できない市区町村。
/// (市区町村コード, P05上の所在地コード, 施設名称)で使うレコードを指定する。
/// 分庁方式などでP05に「◯◯市役所」の名称が無いものは、役場所在地の住所と
/// 一致する庁舎を本庁とした。浜松市の3行政区は2024年1月の再編(7区→3区)が
/// P05-22(2022年)に無いため、旧区の庁舎(現在も同じ建物を使用)を読み替える
const OVERRIDES: [(i32, i32, &str); 13] = [
    (1204, 1204, "総合庁舎"),               // 旭川市(6条通9)
    (1559, 1559, "上湧別庁舎"),             // 湧別町(上湧別屯田市街地318)
    (1610, 1610, "静内庁舎"),               // 新ひだか町(静内御幸町3-2-50)
    (5215, 5215, "仙北市役所田沢湖庁舎"),   // 仙北市(田沢湖生保内字宮ノ後30)
    (9215, 9215, "烏山庁舎"),               // 那須烏山市(中央1-1-1)
    (20452, 20452, "筑北村役場"),           // 筑北村(P05では支所扱い)
    (23234, 23234, "北名古屋市役所西庁舎"), // 北名古屋市(西之保清水田15)
    (24472, 24472, "南伊勢町役場南勢庁舎"), // 南伊勢町(五ヶ所浦3057)
    (32206, 32206, "新安来庁舎"),           // 安来市(安来町878-2)
    (32505, 32505, "吉賀町役場六日市庁舎"), // 吉賀町(六日市750)
    (22138, 22131, "浜松市役所"),           // 浜松市中央区(区役所は市役所本庁舎内)
    (22139, 22136, "浜松市浜北区役所"),     // 浜松市浜名区(旧浜北区役所)
    (22140, 22137, "浜松市天竜区役所"),     // 浜松市天竜区(旧天竜区役所と同一)
];

/// P05の本庁舎レコードの所在地コードのうち、cities.tsvに無くてもよいもの
/// (浜松市の2024年1月再編で消えた旧7区。OVERRIDESで新区へ読み替える)
const OBSOLETE_CODES: [i32; 7] = [22131, 22132, 22133, 22134, 22135, 22136, 22137];

struct CityRecord {
    prefecture_id: i32,
    id: i32,
    designated_city_id: i32,
    name: String,
}

/// P05の施設レコード(分類1=本庁舎、2=支所・出張所等)
struct OfficeRecord {
    location_code: i32,
    category: i32,
    name: String,
    latitude: f64,
    longitude: f64,
}

fn main() {
    let cities = load_cities();
    let offices = load_offices();
    let mut errors = vec![];

    // cities.tsvに無い所在地コードの本庁舎レコードは、既知の旧コード以外ならエラー
    let city_ids: HashSet<i32> = cities.iter().map(|city| city.id).collect();
    for office in offices.iter().filter(|office| office.category == 1) {
        if !city_ids.contains(&office.location_code)
            && !OBSOLETE_CODES.contains(&office.location_code)
        {
            errors.push(format!(
                "cities.tsvに無い所在地コード: {} ({})",
                office.location_code, office.name
            ));
        }
    }

    let mut locations: BTreeMap<i32, (f64, f64)> = BTreeMap::new();
    for city in &cities {
        if NO_OFFICE.contains(&city.id) {
            continue;
        }
        if let Some(&(_, latitude, longitude, _)) =
            MANUAL_LOCATIONS.iter().find(|(id, _, _, _)| *id == city.id)
        {
            locations.insert(city.id, (latitude, longitude));
            continue;
        }
        match resolve(city, &cities, &offices) {
            Ok(office) => {
                locations.insert(city.id, (office.latitude, office.longitude));
            }
            Err(error) => errors.push(format!("{} {}: {}", city.id, city.name, error)),
        }
    }

    for (&id, &(latitude, longitude)) in &locations {
        if !(20.0..=46.0).contains(&latitude) || !(122.0..=154.0).contains(&longitude) {
            errors.push(format!(
                "日本の範囲外の座標: {id} ({latitude}, {longitude})"
            ));
        }
    }

    if !errors.is_empty() {
        for error in &errors {
            eprintln!("{error}");
        }
        eprintln!("{}件のエラーがあります", errors.len());
        exit(1);
    }

    let mut writer = WriterBuilder::new()
        .delimiter(b'\t')
        .from_path("src/data/city_location.tsv")
        .expect("city_location.tsvを開けません");
    for (id, (latitude, longitude)) in &locations {
        writer
            .write_record(&[
                id.to_string(),
                format!("{latitude:.6}"),
                format!("{longitude:.6}"),
            ])
            .expect("city_location.tsvを書き込めません");
    }
    writer.flush().expect("city_location.tsvを書き込めません");
    println!("{}件を書き込みました", locations.len());
}

/// 市区町村に対応する本庁舎レコードを決める。優先順:
/// 1. OVERRIDESで指定されたレコード(所在地コード+名称の完全一致)
/// 2. 自分のコードに所在する本庁舎のうち役場名が一致するもの
/// 3. 同一都道府県内の本庁舎のうち役場名が一致するもの
///    (政令指定都市の市役所は区のコードに、村外役場は所在自治体のコードに付くため)
/// 4. 自分のコードに所在する唯一の本庁舎
///    (政令指定都市の市役所と同居する区役所は市役所の名称でしか入っていないため)
fn resolve<'a>(
    city: &CityRecord,
    cities: &[CityRecord],
    offices: &'a [OfficeRecord],
) -> Result<&'a OfficeRecord, String> {
    if let Some(&(_, location_code, name)) = OVERRIDES.iter().find(|(id, _, _)| *id == city.id) {
        return offices
            .iter()
            .find(|office| office.location_code == location_code && office.name == name)
            .ok_or_else(|| format!("OVERRIDESのレコードが見つかりません: {name}"));
    }

    let expected_names = expected_office_names(city, cities);
    let head_offices = || offices.iter().filter(|office| office.category == 1);
    for expected in &expected_names {
        // 同名施設が表記ゆれで重複することがあるため、正規化なしの一致を優先する
        for normalize in [false, true] {
            let matches: Vec<&OfficeRecord> = head_offices()
                .filter(|office| {
                    office.location_code == city.id
                        && name_matches(&office.name, expected, normalize)
                })
                .collect();
            if let Some(office) = pick_unique(&matches)? {
                return Ok(office);
            }
        }
    }
    for expected in &expected_names {
        for normalize in [false, true] {
            let matches: Vec<&OfficeRecord> = head_offices()
                .filter(|office| {
                    office.location_code / 1000 == city.prefecture_id
                        && name_matches(&office.name, expected, normalize)
                })
                .collect();
            if let Some(office) = pick_unique(&matches)? {
                return Ok(office);
            }
        }
    }
    let located: Vec<&OfficeRecord> = head_offices()
        .filter(|office| office.location_code == city.id)
        .collect();
    match located.as_slice() {
        [office] => Ok(office),
        [] => Err(format!(
            "本庁舎が見つかりません(期待名: {expected_names:?})"
        )),
        _ => Err(format!(
            "本庁舎の候補が複数あります: {:?}",
            located
                .iter()
                .map(|office| &office.name)
                .collect::<Vec<_>>()
        )),
    }
}

/// 市区町村名から期待される役場名の候補を優先順で返す
fn expected_office_names(city: &CityRecord, cities: &[CityRecord]) -> Vec<String> {
    let suffix = if city.name.ends_with('市') || city.name.ends_with('区') {
        "役所"
    } else {
        "役場"
    };
    let mut names = vec![format!("{}{}", city.name, suffix)];
    // 政令指定都市の行政区は市名を省いた「◯◯区役所」の名称でも入っている
    if city.designated_city_id != 0 && city.id != city.designated_city_id {
        if let Some(parent) = cities.iter().find(|c| c.id == city.designated_city_id) {
            if let Some(ward) = city.name.strip_prefix(&parent.name) {
                names.push(format!("{ward}{suffix}"));
            }
        }
    }
    names
}

fn name_matches(name: &str, expected: &str, normalize: bool) -> bool {
    if !normalize {
        return name == expected;
    }
    let fold = |s: &str| s.replace('ヶ', "ケ");
    fold(name) == fold(expected)
}

/// 候補が1件ならそれを、0件ならNoneを返す。複数件は曖昧としてエラー
fn pick_unique<'a>(matches: &[&'a OfficeRecord]) -> Result<Option<&'a OfficeRecord>, String> {
    match matches {
        [] => Ok(None),
        [office] => Ok(Some(office)),
        _ => Err(format!(
            "名称が一致する本庁舎が複数あります: {:?}",
            matches
                .iter()
                .map(|office| (office.location_code, &office.name))
                .collect::<Vec<_>>()
        )),
    }
}

fn load_cities() -> Vec<CityRecord> {
    std::fs::read_to_string("src/data/cities.tsv")
        .expect("src/data/cities.tsvを読めません")
        .lines()
        .map(|line| {
            let mut cols = line.split('\t');
            let mut next = || cols.next().expect("cities.tsv: 列数が不正です");
            CityRecord {
                prefecture_id: next().parse().expect("cities.tsv: prefecture_id"),
                id: next().parse().expect("cities.tsv: id"),
                designated_city_id: next().parse().expect("cities.tsv: designated_city_id"),
                name: next().to_string(),
            }
        })
        .collect()
}

fn load_offices() -> Vec<OfficeRecord> {
    let mut offices = vec![];
    for prefecture_id in 1..=47 {
        let path = format!("storage/p05/P05-22_{prefecture_id:02}.geojson");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{path}を読めません: {error}"));
        for line in content.lines() {
            if !line.trim_start().starts_with(r#"{ "type": "Feature""#) {
                continue;
            }
            let category: i32 = json_string_value(line, "P05_002")
                .unwrap_or_else(|| panic!("{path}: P05_002がありません: {line}"))
                .parse()
                .unwrap_or_else(|_| panic!("{path}: P05_002が数値ではありません: {line}"));
            // 本庁舎(1)と、OVERRIDESが参照する支所・庁舎(2)だけ使う
            if category != 1 && category != 2 {
                continue;
            }
            let (longitude, latitude) = point_coordinates(line)
                .unwrap_or_else(|| panic!("{path}: 座標を読めません: {line}"));
            offices.push(OfficeRecord {
                location_code: json_string_value(line, "P05_001")
                    .unwrap_or_else(|| panic!("{path}: P05_001がありません: {line}"))
                    .parse()
                    .unwrap_or_else(|_| panic!("{path}: P05_001が数値ではありません: {line}")),
                category,
                name: json_string_value(line, "P05_003")
                    .unwrap_or_else(|| panic!("{path}: P05_003がありません: {line}"))
                    .to_string(),
                latitude,
                longitude,
            });
        }
    }
    offices
}

/// GeoJSONの1行から `"key": "value"` の文字列値を取り出す
fn json_string_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!(r#""{key}": ""#);
    let start = line.find(&marker)? + marker.len();
    let end = line[start..].find('"')?;
    Some(&line[start..start + end])
}

/// GeoJSONの1行から `"coordinates": [ 経度, 緯度 ]` を取り出す
fn point_coordinates(line: &str) -> Option<(f64, f64)> {
    let marker = r#""coordinates": ["#;
    let start = line.find(marker)? + marker.len();
    let end = line[start..].find(']')?;
    let mut values = line[start..start + end]
        .split(',')
        .map(|value| value.trim().parse::<f64>());
    let longitude = values.next()?.ok()?;
    let latitude = values.next()?.ok()?;
    Some((longitude, latitude))
}
