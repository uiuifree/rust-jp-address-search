//! 郵便番号住所データ(日本郵便「KEN_ALL」由来)。
//! 検索APIは[`crate::AddressSearch`]に集約している。

use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};
use std::sync::LazyLock;

/// 郵便番号付き住所(日本郵便「KEN_ALL」由来)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address {
    /// 郵便番号(ハイフンなし7桁)
    pub zip: &'static str,
    /// 都道府県コード(JIS X 0401)
    pub prefecture_id: i32,
    /// 市区町村コード(JIS X 0402)
    pub city_id: i32,
    /// 都道府県名(例: `東京都`)
    pub prefecture_name: &'static str,
    /// 市区町村名(例: `千代田区`)
    pub city_name: &'static str,
    /// 町域名(例: `千代田`。町域が特定できない場合は空文字)
    pub town: &'static str,
    /// 町域の補足表記(KEN_ALLの括弧内。丁目・番地の範囲など。例: `1〜19丁目`)
    pub town_note: &'static str,
}

pub(crate) fn all() -> &'static [Address] {
    &ADDRESSES
}

pub(crate) fn preload() {
    LazyLock::force(&ZIP_INDEX);
}

pub(crate) fn find_by_zipcode(zipcode: &str) -> &'static [&'static Address] {
    let mut zip: u32 = 0;
    let mut digit_count = 0;
    for c in zipcode.chars() {
        let Some(digit) = c.to_digit(10) else {
            continue;
        };
        digit_count += 1;
        if digit_count > 7 {
            return &[];
        }
        zip = zip * 10 + digit;
    }
    if digit_count != 7 {
        return &[];
    }
    let index = &*ZIP_INDEX;
    match index.ranges.get(&zip) {
        Some(&(start, len)) => &index.sorted[start as usize..(start + len) as usize],
        None => &[],
    }
}

pub(crate) fn find_by_city_id(city_id: i32) -> &'static [Address] {
    let start = ADDRESSES.partition_point(|address| address.city_id < city_id);
    let end = start + ADDRESSES[start..].partition_point(|address| address.city_id == city_id);
    &ADDRESSES[start..end]
}

// zipcode.tsvは市区町村コード順のため、市区町村コード検索は二分探索で連続範囲を返せる
static ADDRESSES: LazyLock<Vec<Address>> = LazyLock::new(|| {
    let data = include_str!("data/zipcode.tsv");
    let body = &data[data.find('\n').map(|i| i + 1).unwrap_or(data.len())..];

    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(16);
    let chunks = split_at_line_boundaries(body, threads);

    let mut parsed: Vec<Vec<Address>> = vec![];
    std::thread::scope(|scope| {
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| scope.spawn(move || chunk.lines().map(parse_line).collect()))
            .collect();
        parsed = handles
            .into_iter()
            .map(|handle| handle.join().expect("zipcode.tsvのパースに失敗しました"))
            .collect();
    });

    let mut addresses = Vec::with_capacity(parsed.iter().map(Vec::len).sum());
    for mut chunk in parsed {
        addresses.append(&mut chunk);
    }
    addresses
});

/// 行の先頭位置を保ったまま`text`を最大`parts`個に分割する。
fn split_at_line_boundaries(text: &'static str, parts: usize) -> Vec<&'static str> {
    let target = text.len() / parts + 1;
    let mut chunks = vec![];
    let mut rest = text;
    while !rest.is_empty() {
        if rest.len() <= target {
            chunks.push(rest);
            break;
        }
        // targetはマルチバイト文字の途中を指しうるため、バイト列で改行を探す
        let split = rest.as_bytes()[target..]
            .iter()
            .position(|&byte| byte == b'\n')
            .map(|i| target + i + 1)
            .unwrap_or(rest.len());
        let (head, tail) = rest.split_at(split);
        chunks.push(head);
        rest = tail;
    }
    chunks
}

fn parse_line(line: &'static str) -> Address {
    let mut cols = line.split('\t');
    let mut next = || cols.next().expect("zipcode.tsv: 列数が不正です");
    Address {
        zip: next(),
        prefecture_id: next()
            .parse()
            .expect("zipcode.tsv: prefecture_idが数値ではありません"),
        city_id: next()
            .parse()
            .expect("zipcode.tsv: city_idが数値ではありません"),
        prefecture_name: next(),
        city_name: next(),
        town: next(),
        town_note: next(),
    }
}

/// 郵便番号インデックス。全住所への参照をzip順に並べた配列と、
/// zipから配列内の範囲(開始位置, 件数)への表で構成する。
/// 郵便番号ごとに個別の`Vec`を確保するより構築が速く、メモリも少ない。
struct ZipIndex {
    sorted: Vec<&'static Address>,
    ranges: HashMap<u32, (u32, u32), FnvBuildHasher>,
}

static ZIP_INDEX: LazyLock<ZipIndex> = LazyLock::new(|| {
    let mut keyed: Vec<(u32, &'static Address)> = ADDRESSES
        .iter()
        .map(|address| {
            let zip = address
                .zip
                .parse()
                .expect("zipcode.tsv: zipが数値ではありません");
            (zip, address)
        })
        .collect();
    // 安定ソートで同一zip内の元の並び(市区町村コード順)を保つ
    keyed.sort_by_key(|&(zip, _)| zip);

    let mut ranges = HashMap::with_capacity_and_hasher(keyed.len(), FnvBuildHasher);
    let mut start = 0;
    for end in 1..=keyed.len() {
        if end == keyed.len() || keyed[end].0 != keyed[start].0 {
            ranges.insert(keyed[start].0, (start as u32, (end - start) as u32));
            start = end;
        }
    }
    let sorted = keyed.into_iter().map(|(_, address)| address).collect();
    ZipIndex { sorted, ranges }
});

/// FNV-1aハッシュ。キーが信頼できる埋め込みデータのためHashDoS耐性は不要で、
/// 標準のSipHashより短いキーに対して高速。
struct FnvHasher(u64);

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

struct FnvBuildHasher;

impl BuildHasher for FnvBuildHasher {
    type Hasher = FnvHasher;

    fn build_hasher(&self) -> FnvHasher {
        FnvHasher(0xcbf2_9ce4_8422_2325)
    }
}

#[cfg(test)]
mod tests {
    use crate::AddressSearch;

    #[test]
    fn test_addresses() {
        assert_eq!(AddressSearch::addresses().len(), 124496);
    }

    #[test]
    fn test_addresses_sorted_by_city_id() {
        assert!(AddressSearch::addresses()
            .windows(2)
            .all(|pair| pair[0].city_id <= pair[1].city_id));
    }

    #[test]
    fn test_preload() {
        AddressSearch::preload();
        assert!(!AddressSearch::find_by_zipcode("1000001").is_empty());
    }

    #[test]
    fn test_find_by_zipcode() {
        let addresses = AddressSearch::find_by_zipcode("0600041");
        assert_eq!(addresses.len(), 1);
        let address = addresses[0];
        assert_eq!(address.zip, "0600041");
        assert_eq!(address.prefecture_id, 1);
        assert_eq!(address.city_id, 1101);
        assert_eq!(address.prefecture_name, "北海道");
        assert_eq!(address.city_name, "札幌市中央区");
        assert_eq!(address.town, "大通東");
    }

    #[test]
    fn test_find_by_zipcode_with_hyphen() {
        assert_eq!(
            AddressSearch::find_by_zipcode("060-0041"),
            AddressSearch::find_by_zipcode("0600041")
        );
    }

    #[test]
    fn test_find_by_zipcode_multiple_streets() {
        let addresses = AddressSearch::find_by_zipcode("0691329");
        assert_eq!(addresses.len(), 3);
        assert!(addresses.iter().all(|address| address.zip == "0691329"));
    }

    #[test]
    fn test_find_by_zipcode_not_found() {
        assert!(AddressSearch::find_by_zipcode("9999999").is_empty());
        assert!(AddressSearch::find_by_zipcode("番号なし").is_empty());
    }

    #[test]
    fn test_find_by_zipcode_requires_seven_digits() {
        assert!(AddressSearch::find_by_zipcode("600041").is_empty());
        assert!(AddressSearch::find_by_zipcode("06000411").is_empty());
    }

    /// 丁目で郵便番号が分かれる町域はtown_noteで範囲を判別できること
    #[test]
    fn test_town_note() {
        let addresses = AddressSearch::find_by_zipcode("0600042");
        assert_eq!(addresses[0].town, "大通西");
        assert_eq!(addresses[0].town_note, "1〜19丁目");

        let addresses = AddressSearch::find_by_zipcode("0640820");
        assert_eq!(addresses[0].town, "大通西");
        assert_eq!(addresses[0].town_note, "20〜28丁目");
    }

    /// 2024年1月1日の浜松市行政区再編後の住所が引けること
    #[test]
    fn test_find_by_zipcode_hamamatsu() {
        let addresses = AddressSearch::find_by_zipcode("430-0805");
        assert_eq!(addresses.len(), 1);
        let address = addresses[0];
        assert_eq!(address.prefecture_id, 22);
        assert_eq!(address.city_id, 22138);
        assert_eq!(address.prefecture_name, "静岡県");
        assert_eq!(address.city_name, "浜松市中央区");
        assert_eq!(address.town, "相生町");

        let wards = [
            (22138, "浜松市中央区"),
            (22139, "浜松市浜名区"),
            (22140, "浜松市天竜区"),
        ];
        for (city_id, city_name) in wards {
            let addresses = AddressSearch::find_addresses_by_city_id(city_id);
            assert!(!addresses.is_empty());
            assert!(addresses
                .iter()
                .all(|address| address.city_name == city_name));
        }
    }

    /// 「東京都世田谷区太子堂4丁目1-1」のような住所を郵便番号から特定できること。
    /// KEN_ALLの粒度は町域(太子堂)までで、丁目・番地はデータに含まれない。
    #[test]
    fn test_setagaya_taishido() {
        let addresses = AddressSearch::find_by_zipcode("154-0004");
        assert_eq!(addresses.len(), 1);
        let address = addresses[0];
        assert_eq!(address.zip, "1540004");
        assert_eq!(address.prefecture_id, 13);
        assert_eq!(address.city_id, 13112);
        assert_eq!(address.prefecture_name, "東京都");
        assert_eq!(address.city_name, "世田谷区");
        assert_eq!(address.town, "太子堂");

        // 市区町村・都道府県マスタと整合していること
        let city = AddressSearch::find_city_by_id(address.city_id).unwrap();
        assert_eq!(city.name, "世田谷区");
        let prefecture = AddressSearch::find_prefecture_by_id(address.prefecture_id).unwrap();
        assert_eq!(prefecture.name, "東京都");

        // 「太子堂」という町域は全国に複数あるが、郵便番号では一意に特定できる
        let sendai = AddressSearch::find_by_zipcode("982-0013");
        assert_eq!(sendai[0].city_name, "仙台市太白区");
        assert_eq!(sendai[0].town, "太子堂");
    }

    #[test]
    fn test_find_addresses_by_city_id() {
        let addresses = AddressSearch::find_addresses_by_city_id(13101);
        assert_eq!(addresses.len(), 485);
        assert!(addresses.iter().all(|address| address.city_id == 13101));
        assert!(AddressSearch::find_addresses_by_city_id(0).is_empty());
    }
}
