//! 都道府県マスタ(JIS X 0401)。検索APIは[`crate::AddressSearch`]に集約している。

use std::sync::LazyLock;

/// 都道府県
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Prefecture {
    /// 都道府県コード(JIS X 0401)
    pub id: i32,
    /// 名称(例: `東京都`)
    pub name: &'static str,
    /// 短縮名(例: `東京`)
    pub short_name: &'static str,
    /// ローマ字名(例: `tokyo`)
    pub en_name: &'static str,
}

pub(crate) fn all() -> &'static [Prefecture] {
    &PREFECTURES
}

pub(crate) fn find_by_id(id: i32) -> Option<&'static Prefecture> {
    PREFECTURES.iter().find(|prefecture| prefecture.id == id)
}

pub(crate) fn find_by_name(name: &str) -> Option<&'static Prefecture> {
    PREFECTURES
        .iter()
        .find(|prefecture| prefecture.name == name)
}

static PREFECTURES: LazyLock<Vec<Prefecture>> = LazyLock::new(|| {
    include_str!("data/prefectures.tsv")
        .lines()
        .map(|line| {
            let mut cols = line.split('\t');
            let mut next = || cols.next().expect("prefectures.tsv: 列数が不正です");
            Prefecture {
                id: next()
                    .parse()
                    .expect("prefectures.tsv: idが数値ではありません"),
                short_name: next(),
                name: next(),
                en_name: next(),
            }
        })
        .collect()
});

#[cfg(test)]
mod tests {
    use crate::AddressSearch;

    /// (id, 短縮名, 名称, ローマ字名)
    const ALL_PREFECTURES: [(i32, &str, &str, &str); 47] = [
        (1, "北海道", "北海道", "hokkaido"),
        (2, "青森", "青森県", "aomori"),
        (3, "岩手", "岩手県", "iwate"),
        (4, "宮城", "宮城県", "miyagi"),
        (5, "秋田", "秋田県", "akita"),
        (6, "山形", "山形県", "yamagata"),
        (7, "福島", "福島県", "fukushima"),
        (8, "茨城", "茨城県", "ibaraki"),
        (9, "栃木", "栃木県", "tochigi"),
        (10, "群馬", "群馬県", "gunma"),
        (11, "埼玉", "埼玉県", "saitama"),
        (12, "千葉", "千葉県", "chiba"),
        (13, "東京", "東京都", "tokyo"),
        (14, "神奈川", "神奈川県", "kanagawa"),
        (15, "新潟", "新潟県", "niigata"),
        (16, "富山", "富山県", "toyama"),
        (17, "石川", "石川県", "ishikawa"),
        (18, "福井", "福井県", "fukui"),
        (19, "山梨", "山梨県", "yamanashi"),
        (20, "長野", "長野県", "nagano"),
        (21, "岐阜", "岐阜県", "gifu"),
        (22, "静岡", "静岡県", "shizuoka"),
        (23, "愛知", "愛知県", "aichi"),
        (24, "三重", "三重県", "mie"),
        (25, "滋賀", "滋賀県", "shiga"),
        (26, "京都", "京都府", "kyoto"),
        (27, "大阪", "大阪府", "osaka"),
        (28, "兵庫", "兵庫県", "hyogo"),
        (29, "奈良", "奈良県", "nara"),
        (30, "和歌山", "和歌山県", "wakayama"),
        (31, "鳥取", "鳥取県", "tottori"),
        (32, "島根", "島根県", "shimane"),
        (33, "岡山", "岡山県", "okayama"),
        (34, "広島", "広島県", "hiroshima"),
        (35, "山口", "山口県", "yamaguchi"),
        (36, "徳島", "徳島県", "tokushima"),
        (37, "香川", "香川県", "kagawa"),
        (38, "愛媛", "愛媛県", "ehime"),
        (39, "高知", "高知県", "kochi"),
        (40, "福岡", "福岡県", "fukuoka"),
        (41, "佐賀", "佐賀県", "saga"),
        (42, "長崎", "長崎県", "nagasaki"),
        (43, "熊本", "熊本県", "kumamoto"),
        (44, "大分", "大分県", "oita"),
        (45, "宮崎", "宮崎県", "miyazaki"),
        (46, "鹿児島", "鹿児島県", "kagoshima"),
        (47, "沖縄", "沖縄県", "okinawa"),
    ];

    #[test]
    fn test_prefectures() {
        let prefectures = AddressSearch::prefectures();
        assert_eq!(prefectures.len(), 47);
        assert_eq!(prefectures[0].id, 1);
        assert_eq!(prefectures[46].id, 47);
    }

    #[test]
    fn test_all_47_prefectures() {
        for (id, short_name, name, en_name) in ALL_PREFECTURES {
            let prefecture = AddressSearch::find_prefecture_by_id(id)
                .unwrap_or_else(|| panic!("都道府県コード{id}が見つかりません"));
            assert_eq!(prefecture.short_name, short_name);
            assert_eq!(prefecture.name, name);
            assert_eq!(prefecture.en_name, en_name);
            assert_eq!(AddressSearch::find_prefecture_by_name(name).unwrap().id, id);
        }
    }

    #[test]
    fn test_find_prefecture_by_id() {
        let prefecture = AddressSearch::find_prefecture_by_id(13).unwrap();
        assert_eq!(prefecture.id, 13);
        assert_eq!(prefecture.name, "東京都");
        assert_eq!(prefecture.short_name, "東京");
        assert_eq!(prefecture.en_name, "tokyo");
        assert!(AddressSearch::find_prefecture_by_id(0).is_none());
    }

    #[test]
    fn test_find_prefecture_by_name() {
        let prefecture = AddressSearch::find_prefecture_by_name("北海道").unwrap();
        assert_eq!(prefecture.id, 1);
        assert!(AddressSearch::find_prefecture_by_name("東京").is_none());
    }
}
