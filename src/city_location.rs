//! 市区町村の代表点(国土数値情報「市区町村役場等及び公的集会施設データ」由来)。
//! 検索APIは[`crate::AddressSearch`]に集約している。

use std::sync::LazyLock;

/// 市区町村の代表点(本庁舎の位置)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CityLocation {
    /// 市区町村コード(JIS X 0402、検査数字なしの5桁)
    pub city_id: i32,
    /// 本庁舎の緯度(世界測地系の10進度)
    pub latitude: f64,
    /// 本庁舎の経度(世界測地系の10進度)
    pub longitude: f64,
}

pub(crate) fn all() -> &'static [CityLocation] {
    &LOCATIONS
}

pub(crate) fn find_by_city_id(city_id: i32) -> Option<&'static CityLocation> {
    LOCATIONS
        .binary_search_by_key(&city_id, |location| location.city_id)
        .ok()
        .map(|index| &LOCATIONS[index])
}

static LOCATIONS: LazyLock<Vec<CityLocation>> = LazyLock::new(|| {
    include_str!("data/city_location.tsv")
        .lines()
        .map(|line| {
            let mut cols = line.split('\t');
            let mut next = || cols.next().expect("city_location.tsv: 列数が不正です");
            CityLocation {
                city_id: next()
                    .parse()
                    .expect("city_location.tsv: city_idが数値ではありません"),
                latitude: next()
                    .parse()
                    .expect("city_location.tsv: latitudeが数値ではありません"),
                longitude: next()
                    .parse()
                    .expect("city_location.tsv: longitudeが数値ではありません"),
            }
        })
        .collect()
});

#[cfg(test)]
mod tests {
    use super::all;
    use crate::AddressSearch;

    /// 役場が実在しないため座標を持たない市区町村(北方領土の6村)
    const NO_OFFICE: [i32; 6] = [1695, 1696, 1697, 1698, 1699, 1700];

    #[test]
    fn test_city_locations_cover_all_cities_except_no_office() {
        assert_eq!(all().len(), 1912);
        assert_eq!(all().len(), AddressSearch::cities().len() - NO_OFFICE.len());

        for city in AddressSearch::cities() {
            let location = AddressSearch::find_city_location(city.id);
            if NO_OFFICE.contains(&city.id) {
                assert!(
                    location.is_none(),
                    "{}({})に座標があります",
                    city.id,
                    city.name
                );
            } else {
                assert!(
                    location.is_some(),
                    "{}({})に座標がありません",
                    city.id,
                    city.name
                );
            }
        }
    }

    #[test]
    fn test_city_locations_sorted_by_city_id() {
        assert!(all()
            .windows(2)
            .all(|pair| pair[0].city_id < pair[1].city_id));
    }

    /// 全座標が日本の範囲(緯度20〜46 / 経度122〜154)に収まっていること
    #[test]
    fn test_city_locations_within_japan() {
        for location in all() {
            assert!(
                (20.0..=46.0).contains(&location.latitude)
                    && (122.0..=154.0).contains(&location.longitude),
                "{}の座標が日本の範囲外です: ({}, {})",
                location.city_id,
                location.latitude,
                location.longitude
            );
        }
    }

    #[test]
    fn test_find_city_location() {
        // 渋谷区 → 渋谷区役所
        let shibuya = AddressSearch::find_city_location(13113).unwrap();
        assert!((shibuya.latitude - 35.664).abs() < 0.01);
        assert!((shibuya.longitude - 139.698).abs() < 0.01);

        assert!(AddressSearch::find_city_location(0).is_none());
        assert!(AddressSearch::find_city_location(99999).is_none());
    }

    /// 政令指定都市は本体(市役所)と行政区(区役所)の両方に座標があること
    #[test]
    fn test_designated_city_and_wards() {
        // 札幌市(市役所)と札幌市中央区(区役所)は別の建物
        let sapporo = AddressSearch::find_city_location(1100).unwrap();
        let chuo = AddressSearch::find_city_location(1101).unwrap();
        assert_ne!(
            (sapporo.latitude, sapporo.longitude),
            (chuo.latitude, chuo.longitude)
        );

        // 静岡市役所は葵区役所と同居しているため同一座標
        let shizuoka = AddressSearch::find_city_location(22100).unwrap();
        let aoi = AddressSearch::find_city_location(22101).unwrap();
        assert_eq!(
            (shizuoka.latitude, shizuoka.longitude),
            (aoi.latitude, aoi.longitude)
        );
    }

    /// 2024年1月再編後の浜松市3行政区に座標があること
    #[test]
    fn test_hamamatsu_reorganized_wards() {
        for id in [22130, 22138, 22139, 22140] {
            assert!(AddressSearch::find_city_location(id).is_some());
        }
        // 再編前の旧区コードには無い
        for old_id in [22131, 22132, 22133, 22134, 22135, 22136, 22137] {
            assert!(AddressSearch::find_city_location(old_id).is_none());
        }
        // 浜名区役所(旧浜北区役所) ≈ 34.792, 137.783
        let hamana = AddressSearch::find_city_location(22139).unwrap();
        assert!((hamana.latitude - 34.792).abs() < 0.01);
        assert!((hamana.longitude - 137.783).abs() < 0.01);
    }

    /// 役場が自治体の外にある場合は役場の実際の所在地を返すこと
    #[test]
    fn test_office_outside_own_municipality() {
        // 竹富町役場は石垣市にある
        let taketomi = AddressSearch::find_city_location(47381).unwrap();
        assert!((taketomi.latitude - 24.340).abs() < 0.01);
        assert!((taketomi.longitude - 124.155).abs() < 0.01);
    }
}
