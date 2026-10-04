//! 総務省「全国地方公共団体コード」のExcelから `src/data/cities.tsv` を生成する。
//!
//! 入力: `./storage/code.xlsx`
//! (https://www.soumu.go.jp/denshijiti/code.html の「都道府県コード及び市区町村コード」)
//! 実行: `cargo run --features generate --bin update_master`

use calamine::{open_workbook, Data, Reader, Xlsx};
use csv::WriterBuilder;
use std::collections::HashSet;

struct CityRecord {
    prefecture_id: i32,
    id: i32,
    designated_city_id: i32,
    name: String,
}

fn main() {
    let mut excel: Xlsx<_> =
        open_workbook("./storage/code.xlsx").expect("storage/code.xlsxを開けません");
    // シート名には更新日が含まれる(例: `R6.1.1政令指定都市`)ため部分一致で探す
    let sheet_names = excel.sheet_names().to_owned();
    let major_city_sheet = sheet_names
        .iter()
        .find(|name| name.contains("政令指定都市"))
        .expect("「政令指定都市」シートが見つかりません")
        .clone();
    let city_sheet = sheet_names
        .iter()
        .find(|name| name.contains("現在の団体"))
        .expect("「現在の団体」シートが見つかりません")
        .clone();

    let mut cities = vec![];
    let mut seen_ids = HashSet::new();

    // 政令指定都市とその行政区。市の行を親として、続く区の行にdesignated_city_idを付与する
    let range = excel
        .worksheet_range(&major_city_sheet)
        .expect("「政令指定都市」シートを読み込めません");
    let mut designated_city_id = 0;
    for row in range.rows().skip(1) {
        let Some(mut city) = row_to_city(row) else {
            continue;
        };
        if city.name.ends_with('市') {
            designated_city_id = city.id;
        }
        city.designated_city_id = designated_city_id;
        seen_ids.insert(city.id);
        cities.push(city);
    }

    let range = excel
        .worksheet_range(&city_sheet)
        .expect("「現在の団体」シートを読み込めません");
    for row in range.rows().skip(1) {
        let Some(city) = row_to_city(row) else {
            continue;
        };
        if !seen_ids.insert(city.id) {
            continue;
        }
        cities.push(city);
    }

    cities.sort_by_key(|city| city.id);
    let mut writer = WriterBuilder::new()
        .delimiter(b'\t')
        .from_path("src/data/cities.tsv")
        .expect("cities.tsvを開けません");
    for city in cities {
        writer
            .write_record(&[
                city.prefecture_id.to_string(),
                city.id.to_string(),
                city.designated_city_id.to_string(),
                city.name,
            ])
            .expect("cities.tsvを書き込めません");
    }
    writer.flush().expect("cities.tsvを書き込めません");
}

fn row_to_city(row: &[Data]) -> Option<CityRecord> {
    let name = row[2].to_string();
    if name.is_empty() {
        return None;
    }
    // 1列目は6桁の団体コード(5桁のJISコード+検査数字1桁)
    let code: i32 = row[0]
        .to_string()
        .parse()
        .expect("団体コードが数値ではありません");
    let id = code / 10;
    Some(CityRecord {
        prefecture_id: id / 1000,
        id,
        designated_city_id: 0,
        name,
    })
}
