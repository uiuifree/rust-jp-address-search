//! 市区町村マスタ(総務省「全国地方公共団体コード」由来)。
//! 検索APIは[`crate::AddressSearch`]に集約している。

use std::sync::LazyLock;

/// 市区町村
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct City {
    /// 市区町村コード(JIS X 0402、検査数字なしの5桁)
    pub id: i32,
    /// 都道府県コード(JIS X 0401)
    pub prefecture_id: i32,
    /// 政令指定都市のコード(政令指定都市とその行政区のみ。それ以外は0)
    pub designated_city_id: i32,
    /// 名称(例: `札幌市中央区`)
    pub name: &'static str,
}

pub(crate) fn all() -> &'static [City] {
    &CITIES
}

pub(crate) fn find_by_id(id: i32) -> Option<&'static City> {
    CITIES
        .binary_search_by_key(&id, |city| city.id)
        .ok()
        .map(|index| &CITIES[index])
}

pub(crate) fn find_by_name(name: &str) -> Option<&'static City> {
    CITIES.iter().find(|city| city.name == name)
}

pub(crate) fn find_by_prefecture_id(prefecture_id: i32) -> &'static [City] {
    let start = CITIES.partition_point(|city| city.prefecture_id < prefecture_id);
    let end = start + CITIES[start..].partition_point(|city| city.prefecture_id == prefecture_id);
    &CITIES[start..end]
}

static CITIES: LazyLock<Vec<City>> = LazyLock::new(|| {
    include_str!("data/cities.tsv")
        .lines()
        .map(|line| {
            let mut cols = line.split('\t');
            let mut next = || cols.next().expect("cities.tsv: 列数が不正です");
            City {
                prefecture_id: next()
                    .parse()
                    .expect("cities.tsv: prefecture_idが数値ではありません"),
                id: next().parse().expect("cities.tsv: idが数値ではありません"),
                designated_city_id: next()
                    .parse()
                    .expect("cities.tsv: designated_city_idが数値ではありません"),
                name: next(),
            }
        })
        .collect()
});

#[cfg(test)]
mod tests {
    use crate::AddressSearch;

    /// (都道府県コード, 市区町村数, 県庁所在地の市区町村コード, 県庁所在地名)
    /// 東京都は都庁所在地の特別区(新宿区)
    const CITIES_PER_PREFECTURE: [(i32, usize, i32, &str); 47] = [
        (1, 195, 1100, "札幌市"),
        (2, 40, 2201, "青森市"),
        (3, 33, 3201, "盛岡市"),
        (4, 40, 4100, "仙台市"),
        (5, 25, 5201, "秋田市"),
        (6, 35, 6201, "山形市"),
        (7, 59, 7201, "福島市"),
        (8, 44, 8201, "水戸市"),
        (9, 25, 9201, "宇都宮市"),
        (10, 35, 10201, "前橋市"),
        (11, 73, 11100, "さいたま市"),
        (12, 60, 12100, "千葉市"),
        (13, 62, 13104, "新宿区"),
        (14, 61, 14100, "横浜市"),
        (15, 38, 15100, "新潟市"),
        (16, 15, 16201, "富山市"),
        (17, 19, 17201, "金沢市"),
        (18, 17, 18201, "福井市"),
        (19, 27, 19201, "甲府市"),
        (20, 77, 20201, "長野市"),
        (21, 42, 21201, "岐阜市"),
        (22, 41, 22100, "静岡市"),
        (23, 70, 23100, "名古屋市"),
        (24, 29, 24201, "津市"),
        (25, 19, 25201, "大津市"),
        (26, 37, 26100, "京都市"),
        (27, 74, 27100, "大阪市"),
        (28, 50, 28100, "神戸市"),
        (29, 39, 29201, "奈良市"),
        (30, 30, 30201, "和歌山市"),
        (31, 19, 31201, "鳥取市"),
        (32, 19, 32201, "松江市"),
        (33, 31, 33100, "岡山市"),
        (34, 31, 34100, "広島市"),
        (35, 19, 35203, "山口市"),
        (36, 24, 36201, "徳島市"),
        (37, 17, 37201, "高松市"),
        (38, 20, 38201, "松山市"),
        (39, 34, 39201, "高知市"),
        (40, 74, 40130, "福岡市"),
        (41, 20, 41201, "佐賀市"),
        (42, 21, 42201, "長崎市"),
        (43, 50, 43100, "熊本市"),
        (44, 18, 44201, "大分市"),
        (45, 26, 45201, "宮崎市"),
        (46, 43, 46201, "鹿児島市"),
        (47, 41, 47201, "那覇市"),
    ];

    #[test]
    fn test_cities() {
        let cities = AddressSearch::cities();
        assert_eq!(cities.len(), 1918);
    }

    #[test]
    fn test_cities_of_all_47_prefectures() {
        for (prefecture_id, city_count, capital_id, capital_name) in CITIES_PER_PREFECTURE {
            let cities = AddressSearch::find_cities_by_prefecture_id(prefecture_id);
            assert_eq!(
                cities.len(),
                city_count,
                "都道府県コード{prefecture_id}の市区町村数が一致しません"
            );
            assert!(cities
                .iter()
                .all(|city| city.prefecture_id == prefecture_id));

            let capital = AddressSearch::find_city_by_id(capital_id)
                .unwrap_or_else(|| panic!("市区町村コード{capital_id}が見つかりません"));
            assert_eq!(capital.name, capital_name);
            assert_eq!(capital.prefecture_id, prefecture_id);
        }

        // 47都道府県で全市区町村を網羅している
        let total: usize = CITIES_PER_PREFECTURE
            .iter()
            .map(|(_, city_count, _, _)| city_count)
            .sum();
        assert_eq!(total, AddressSearch::cities().len());
    }

    #[test]
    fn test_cities_sorted_by_id() {
        assert!(AddressSearch::cities()
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id));
    }

    #[test]
    fn test_find_city_by_id() {
        let city = AddressSearch::find_city_by_id(13101).unwrap();
        assert_eq!(city.id, 13101);
        assert_eq!(city.prefecture_id, 13);
        assert_eq!(city.name, "千代田区");
        assert!(AddressSearch::find_city_by_id(0).is_none());
    }

    #[test]
    fn test_find_city_by_name() {
        let city = AddressSearch::find_city_by_name("堺市堺区").unwrap();
        assert_eq!(city.id, 27141);
        assert_eq!(city.designated_city_id, 27140);
        assert!(AddressSearch::find_city_by_name("存在しない市").is_none());
    }

    /// 同名の市区町村が複数の都道府県に存在する場合、コード最小のものが返ること
    #[test]
    fn test_find_city_by_name_duplicated_names() {
        // 府中市: 東京都(13206)と広島県(34208)
        let fuchu = AddressSearch::find_city_by_name("府中市").unwrap();
        assert_eq!((fuchu.id, fuchu.prefecture_id), (13206, 13));

        // 伊達市: 北海道(1233)と福島県(7213)
        let date = AddressSearch::find_city_by_name("伊達市").unwrap();
        assert_eq!((date.id, date.prefecture_id), (1233, 1));

        // 広島県の府中市は都道府県で絞り込んで特定できる
        let hiroshima_fuchu = AddressSearch::find_cities_by_prefecture_id(34)
            .iter()
            .find(|city| city.name == "府中市")
            .unwrap();
        assert_eq!(hiroshima_fuchu.id, 34208);
    }

    #[test]
    fn test_find_cities_by_prefecture_id() {
        let cities = AddressSearch::find_cities_by_prefecture_id(13);
        assert!(!cities.is_empty());
        assert!(cities.iter().all(|city| city.prefecture_id == 13));
        assert!(AddressSearch::find_cities_by_prefecture_id(0).is_empty());
    }

    /// 2024年1月1日の浜松市行政区再編(7区→3区)が反映されていること
    #[test]
    fn test_hamamatsu_reorganized_wards() {
        let hamamatsu = AddressSearch::find_city_by_id(22130).unwrap();
        assert_eq!(hamamatsu.name, "浜松市");
        assert_eq!(hamamatsu.prefecture_id, 22);
        assert_eq!(hamamatsu.designated_city_id, 22130);

        for (id, name) in [
            (22138, "浜松市中央区"),
            (22139, "浜松市浜名区"),
            (22140, "浜松市天竜区"),
        ] {
            let ward = AddressSearch::find_city_by_id(id)
                .unwrap_or_else(|| panic!("市区町村コード{id}が見つかりません"));
            assert_eq!(ward.name, name);
            assert_eq!(ward.prefecture_id, 22);
            assert_eq!(ward.designated_city_id, 22130);
        }

        // 再編前の7区のコードは存在しない
        for old_id in [22131, 22132, 22133, 22134, 22135, 22136, 22137] {
            assert!(AddressSearch::find_city_by_id(old_id).is_none());
        }
    }
}
