//! 住所文字列の解析。検索APIは[`crate::AddressSearch`]に集約している。
//!
//! 「都道府県名 → 市区町村名 → 町域名」の順に埋め込みデータへの
//! 最長前方一致で解析する。

use crate::{address, city, prefecture, Address, City, Prefecture};
use std::sync::LazyLock;

/// [`crate::AddressSearch::parse_address`]の結果。
/// 特定できた階層のフィールドにのみ値が入る。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressMatch {
    /// 一致した都道府県
    pub prefecture: Option<&'static Prefecture>,
    /// 一致した市区町村
    pub city: Option<&'static City>,
    /// 町域まで一致した住所(郵便番号の特定に使える)
    pub address: Option<&'static Address>,
}

pub(crate) fn parse_address(input: &str) -> AddressMatch {
    let normalized = normalize(input);
    let mut rest = normalized.as_str();

    let mut prefecture = prefecture::all()
        .iter()
        .find(|prefecture| match_prefix(rest, prefecture.name).is_some());
    if let Some(prefecture) = prefecture {
        rest = &rest[prefecture.name.len()..];
    }

    let Some((city, consumed)) = match_city(rest, prefecture) else {
        return AddressMatch {
            prefecture,
            city: None,
            address: None,
        };
    };
    rest = &rest[consumed..];
    if prefecture.is_none() {
        prefecture = prefecture::find_by_id(city.prefecture_id);
    }

    AddressMatch {
        prefecture,
        city: Some(city),
        address: match_town(rest, city),
    }
}

/// 空白を除去し、全角数字を半角にする。
fn normalize(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '０'..='９' => char::from_u32(c as u32 - '０' as u32 + '0' as u32).unwrap(),
            _ => c,
        })
        .collect()
}

/// 表記ゆれの正規化。ヶ・ケ・が は同一視する(例: 霞ヶ関/霞が関、旭ケ丘/旭ヶ丘)。
fn canonical(c: char) -> char {
    match c {
        'ヶ' | 'が' => 'ケ',
        _ => c,
    }
}

/// `name`が`input`の先頭に(表記ゆれを許容して)一致する場合、
/// `input`側の消費バイト数を返す。
fn match_prefix(input: &str, name: &str) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    let mut input_chars = input.char_indices();
    for name_char in name.chars() {
        let (_, input_char) = input_chars.next()?;
        if canonical(input_char) != canonical(name_char) {
            return None;
        }
    }
    Some(input_chars.next().map(|(i, _)| i).unwrap_or(input.len()))
}

/// 市区町村を最長前方一致で探す。一致した場合は消費バイト数も返す。
/// 名称は市区町村マスタ(郡名なし)とKEN_ALL表記(郡名付き)の両方を候補にする。
fn match_city(
    rest: &str,
    prefecture: Option<&'static Prefecture>,
) -> Option<(&'static City, usize)> {
    let mut best: Option<(&'static City, usize)> = None;

    let masters = match prefecture {
        Some(prefecture) => city::find_by_prefecture_id(prefecture.id),
        None => city::all(),
    };
    for city in masters {
        consider(&mut best, rest, city, city.name);
    }

    for &(city_id, name) in KENALL_CITY_NAMES.iter() {
        if let Some(prefecture) = prefecture {
            if city_id / 1000 != prefecture.id {
                continue;
            }
        }
        if let Some(city) = city::find_by_id(city_id) {
            consider(&mut best, rest, city, name);
        }
    }
    best
}

fn consider(
    best: &mut Option<(&'static City, usize)>,
    rest: &str,
    city: &'static City,
    name: &str,
) {
    let Some(consumed) = match_prefix(rest, name) else {
        return;
    };
    let replace = match *best {
        None => true,
        // 最長一致を優先し、同名の市区町村はコード最小を返す
        Some((current, current_consumed)) => {
            consumed > current_consumed || (consumed == current_consumed && city.id < current.id)
        }
    };
    if replace {
        *best = Some((city, consumed));
    }
}

/// 町域を最長前方一致で探す。「大字」は省いて再試行する。
/// 一致する町域がない場合は「以下に掲載がない場合」の住所(町域が空)を返す。
fn match_town(rest: &str, city: &'static City) -> Option<&'static Address> {
    if rest.is_empty() {
        return None;
    }
    let addresses = address::find_by_city_id(city.id);
    let matched = longest_town_match(rest, addresses).or_else(|| {
        rest.strip_prefix("大字")
            .and_then(|stripped| longest_town_match(stripped, addresses))
    });
    matched.or_else(|| addresses.iter().find(|address| address.town.is_empty()))
}

fn longest_town_match(rest: &str, addresses: &'static [Address]) -> Option<&'static Address> {
    let mut best: Option<(&'static Address, usize)> = None;
    for address in addresses {
        let Some(consumed) = match_prefix(rest, address.town) else {
            continue;
        };
        let longer = match best {
            None => true,
            Some((current, _)) => address.town.len() > current.town.len(),
        };
        if longer {
            best = Some((address, consumed));
        }
    }
    let (first, consumed) = best?;

    // 同名の町域が丁目で複数の郵便番号に分かれる場合(札幌市中央区「大通西」など)は、
    // 町域に続く数字を丁目とみなしtown_noteの範囲と照合する
    let candidates = addresses
        .iter()
        .filter(|address| address.town == first.town);
    if let Some(chome) = leading_number(&rest[consumed..]) {
        if let Some(matched) = candidates
            .into_iter()
            .find(|address| note_contains_chome(address.town_note, chome))
        {
            return Some(matched);
        }
    }
    Some(first)
}

/// 先頭の連続した数字を返す(例: `25丁目1-1`→25)。
fn leading_number(rest: &str) -> Option<u32> {
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// 補足表記(例: `1〜19丁目`、`1、2丁目`)が丁目`chome`を含むかを判定する。
fn note_contains_chome(note: &str, chome: u32) -> bool {
    let Some(chome_index) = note.find("丁目") else {
        return false;
    };
    // 「丁目」直前の数字・範囲・列挙の並びだけを対象にする
    let head: Vec<char> = note[..chome_index].chars().collect();
    let expr_start = head
        .iter()
        .rposition(|&c| !(c.is_ascii_digit() || matches!(c, '〜' | '～' | '-' | '−' | '、')))
        .map(|i| i + 1)
        .unwrap_or(0);
    let expr: String = head[expr_start..].iter().collect();

    let numbers: Vec<u32> = expr
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|digits| digits.parse().ok())
        .collect();
    let (Some(&min), Some(&max)) = (numbers.first(), numbers.last()) else {
        return false;
    };
    if expr.contains(['〜', '～', '-', '−']) {
        (min..=max).contains(&chome)
    } else {
        numbers.contains(&chome)
    }
}

/// KEN_ALL表記の市区町村名(郡名付き。例: `夕張郡長沼町`)。
/// 住所データは市区町村コード順のため、コードの切り替わりで一意に集められる。
static KENALL_CITY_NAMES: LazyLock<Vec<(i32, &'static str)>> = LazyLock::new(|| {
    let mut names = vec![];
    let mut last_city_id = 0;
    for address in address::all() {
        if address.city_id != last_city_id {
            names.push((address.city_id, address.city_name));
            last_city_id = address.city_id;
        }
    }
    names
});

#[cfg(test)]
mod tests {
    use crate::AddressSearch;

    #[test]
    fn test_full_address() {
        let matched = AddressSearch::parse_address("東京都世田谷区太子堂4丁目1-1");
        assert_eq!(matched.prefecture.unwrap().name, "東京都");
        assert_eq!(matched.city.unwrap().id, 13112);
        let address = matched.address.unwrap();
        assert_eq!(address.zip, "1540004");
        assert_eq!(address.town, "太子堂");
    }

    #[test]
    fn test_without_prefecture() {
        let matched = AddressSearch::parse_address("世田谷区太子堂4-1-1");
        assert_eq!(matched.prefecture.unwrap().name, "東京都");
        assert_eq!(matched.city.unwrap().name, "世田谷区");
        assert_eq!(matched.address.unwrap().zip, "1540004");
    }

    #[test]
    fn test_county_name() {
        // 郡名付き(KEN_ALL表記)
        let matched = AddressSearch::parse_address("北海道夕張郡長沼町木詰");
        assert_eq!(matched.city.unwrap().name, "長沼町");
        assert_eq!(matched.address.unwrap().zip, "0691329");

        // 郡名なし(マスタ表記)
        let matched = AddressSearch::parse_address("北海道長沼町木詰");
        assert_eq!(matched.address.unwrap().zip, "0691329");
    }

    #[test]
    fn test_kana_variants() {
        // データは「霞が関」だが「霞ヶ関」でも一致する
        let matched = AddressSearch::parse_address("東京都千代田区霞ヶ関1丁目");
        assert_eq!(matched.address.unwrap().zip, "1000013");
    }

    #[test]
    fn test_oaza() {
        let matched = AddressSearch::parse_address("北海道夕張郡長沼町大字木詰");
        assert_eq!(matched.address.unwrap().zip, "0691329");
    }

    /// 丁目で郵便番号が分かれる町域を丁目で特定できること
    #[test]
    fn test_chome_disambiguation() {
        // 札幌市中央区大通西: 1〜19丁目=0600042、20〜28丁目=0640820
        let matched = AddressSearch::parse_address("北海道札幌市中央区大通西4丁目1-1");
        assert_eq!(matched.address.unwrap().zip, "0600042");

        let matched = AddressSearch::parse_address("北海道札幌市中央区大通西25丁目");
        assert_eq!(matched.address.unwrap().zip, "0640820");

        // 丁目の記載がない場合は先頭の候補を返す
        let matched = AddressSearch::parse_address("北海道札幌市中央区大通西");
        assert_eq!(matched.address.unwrap().zip, "0600042");
    }

    #[test]
    fn test_unlisted_street_falls_back_to_default_zip() {
        // 町域が一覧にない場合は「以下に掲載がない場合」の郵便番号
        let matched = AddressSearch::parse_address("北海道千歳市見知らぬ町3-4");
        assert_eq!(matched.city.unwrap().name, "千歳市");
        let address = matched.address.unwrap();
        assert_eq!(address.zip, "0660000");
        assert_eq!(address.town, "");
    }

    #[test]
    fn test_designated_city_ward() {
        let matched = AddressSearch::parse_address("神奈川県横浜市西区みなとみらい2丁目");
        let city = matched.city.unwrap();
        assert_eq!(city.id, 14103);
        assert_eq!(city.designated_city_id, 14100);
        assert_eq!(matched.address.unwrap().zip, "2200000");
    }

    #[test]
    fn test_duplicated_city_name() {
        // 府中市は東京都と広島県に存在し、コード最小の東京都が返る
        let matched = AddressSearch::parse_address("府中市");
        assert_eq!(matched.prefecture.unwrap().name, "東京都");
        assert_eq!(matched.city.unwrap().id, 13206);
        assert!(matched.address.is_none());
    }

    #[test]
    fn test_prefecture_only() {
        let matched = AddressSearch::parse_address("東京都");
        assert_eq!(matched.prefecture.unwrap().id, 13);
        assert!(matched.city.is_none());
        assert!(matched.address.is_none());
    }

    #[test]
    fn test_not_found() {
        let matched = AddressSearch::parse_address("存在しない住所1-2-3");
        assert!(matched.prefecture.is_none());
        assert!(matched.city.is_none());
        assert!(matched.address.is_none());
    }

    #[test]
    fn test_whitespace_and_full_width() {
        let matched = AddressSearch::parse_address("東京都 世田谷区 太子堂４丁目");
        assert_eq!(matched.address.unwrap().zip, "1540004");
    }
}
