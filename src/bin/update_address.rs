//! 日本郵便「KEN_ALL(住所の郵便番号)」のCSVから `src/data/zipcode.tsv` を生成する。
//!
//! 入力: `./storage/utf_ken_all.csv`(1レコード1行・UTF-8形式)
//! (https://www.post.japanpost.jp/service/search/zipcode/download/utf-zip.html)
//! 実行: `cargo run --features generate --bin update_address`

use csv::{ReaderBuilder, WriterBuilder};
use std::collections::HashSet;

fn main() {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .from_path("./storage/utf_ken_all.csv")
        .expect("storage/utf_ken_all.csvを開けません");

    let mut seen_keys = HashSet::new();
    let mut addresses = vec![];
    for record in reader.records() {
        let record = record.expect("KEN_ALL_UTF8.csvを読み込めません");
        // 1列目は5桁の全国地方公共団体コード(上2桁が都道府県コード)
        let city_id: i32 = record[0]
            .parse()
            .expect("全国地方公共団体コードが数値ではありません");
        let zip = &record[2];
        let prefecture_name = &record[6];
        let city_name = &record[7];
        let (town, town_note) = parse_town(&record[8], &record[11]);

        let key = format!("{zip}{prefecture_name}{city_name}{town}");
        if !seen_keys.insert(key) {
            continue;
        }
        addresses.push([
            zip.to_string(),
            (city_id / 1000).to_string(),
            city_id.to_string(),
            prefecture_name.to_string(),
            city_name.to_string(),
            town,
            town_note,
        ]);
    }

    let mut writer = WriterBuilder::new()
        .delimiter(b'\t')
        .from_path("src/data/zipcode.tsv")
        .expect("zipcode.tsvを開けません");
    writer
        .write_record([
            "zip",
            "prefecture_id",
            "city_id",
            "prefecture_name",
            "city_name",
            "town",
            "town_note",
        ])
        .expect("zipcode.tsvを書き込めません");
    for address in addresses {
        writer
            .write_record(&address)
            .expect("zipcode.tsvを書き込めません");
    }
    writer.flush().expect("zipcode.tsvを書き込めません");
}

/// KEN_ALLの町域名を検索用の町域名と補足表記(丁目・番地の範囲など)に分ける。
///
/// - 全角数字は半角にする
/// - 「以下に掲載がない場合」は町域を特定できないため空にする
/// - 括弧内の表記は、番地(`is_street_no == "1"`)・数字・列挙(読点)を
///   含まない場合のみ町域名の一部として残し、それ以外は補足表記として保持する
/// - 複数の町域が読点で列挙されている場合は特定できないため空にする
fn parse_town(raw: &str, is_street_no: &str) -> (String, String) {
    let normalized: String = raw
        .chars()
        .map(|c| match c {
            '０'..='９' => char::from_u32(c as u32 - '０' as u32 + '0' as u32).unwrap(),
            _ => c,
        })
        .collect();
    if normalized == "以下に掲載がない場合" {
        return (String::new(), String::new());
    }

    // 括弧で区切った先頭2セグメントのみを対象にする
    let mut segments = normalized.split('（');
    let mut street = segments.next().unwrap_or_default().to_string();
    let mut note = String::new();
    if let Some(paren) = segments.next() {
        let appendable = is_street_no != "1"
            && !paren.contains('、')
            && !paren.contains(|c: char| c.is_ascii_digit());
        if appendable {
            street.push_str(paren);
        } else {
            // 「その他」は補足として無意味なため読点区切りで除去する
            note = paren
                .trim_end_matches('）')
                .split('、')
                .filter(|segment| *segment != "その他")
                .collect::<Vec<_>>()
                .join("、");
        }
    }

    let street = street.replace(['（', '）'], "").replace("その他", "");
    if street.contains('、') {
        return (String::new(), note);
    }
    (street, note)
}
