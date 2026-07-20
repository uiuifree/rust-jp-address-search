//! 基本的な使い方の例。実行: `cargo run --example search`

use jp_address_search::AddressSearch;

fn main() {
    // 郵便番号 → 住所
    for address in AddressSearch::find_by_zipcode("154-0004") {
        println!(
            "〒{} {}{}{}",
            address.zip, address.prefecture_name, address.city_name, address.town
        );
    }

    // 都道府県 → 市区町村一覧
    let tokyo = AddressSearch::find_prefecture_by_name("東京都").unwrap();
    let cities = AddressSearch::find_cities_by_prefecture_id(tokyo.id);
    println!("{}の市区町村: {}件", tokyo.name, cities.len());

    // 名称 → 市区町村(同名の場合はコード最小のものが返る)
    let fuchu = AddressSearch::find_city_by_name("府中市").unwrap();
    let prefecture = AddressSearch::find_prefecture_by_id(fuchu.prefecture_id).unwrap();
    println!("{}{} ({})", prefecture.name, fuchu.name, fuchu.id);

    // 住所文字列 → 都道府県・市区町村・郵便番号
    let matched = AddressSearch::parse_address("東京都世田谷区太子堂4丁目1-1");
    if let Some(address) = matched.address {
        println!("〒{} が特定できました", address.zip);
    }
}
