//! 日本の住所データ(都道府県・市区町村・郵便番号)を検索するライブラリ。
//! Fast, zero-dependency Japanese address search by postal code (zipcode),
//! prefecture, and city — all data embedded at compile time.
//!
//! すべての検索は[`AddressSearch`]に集約されている。
//!
//! # 特徴
//!
//! - **自己完結**: すべてのデータをバイナリに埋め込むため、実行時に外部ファイルや
//!   ネットワークへアクセスしない。ランタイム依存クレートもなし
//! - **高速**: 郵便番号検索は数ns(ハッシュ索引)、市区町村コード検索は
//!   二分探索。検索時にヒープ確保を行わず、結果は`'static`スライスで返す
//! - **遅延読み込み**: データは初回アクセス時に構築される(数ms〜数十ms)。
//!   [`AddressSearch::preload`]で起動時に済ませることもできる
//!
//! # データ
//!
//! - [`Prefecture`][] — 都道府県マスタ(47件)
//! - [`City`][] — 市区町村マスタ(総務省「全国地方公共団体コード」由来)
//! - [`Address`][] — 郵便番号住所データ(日本郵便「KEN_ALL」由来)
//!
//! # 使用例
//!
//! ```
//! use jp_address_search::AddressSearch;
//!
//! let prefecture = AddressSearch::find_prefecture_by_name("東京都").unwrap();
//! assert_eq!(prefecture.id, 13);
//!
//! let city = AddressSearch::find_city_by_id(13101).unwrap();
//! assert_eq!(city.name, "千代田区");
//!
//! let addresses = AddressSearch::find_by_zipcode("100-0001");
//! assert_eq!(addresses[0].city_name, "千代田区");
//! assert_eq!(addresses[0].town, "千代田");
//! ```
//!
//! # データの更新
//!
//! 埋め込みデータの再生成には`generate` featureのバイナリを使う。
//! 手順はリポジトリのREADMEを参照。

#![warn(missing_docs)]

mod address;
mod city;
mod parse;
mod prefecture;

pub use address::Address;
pub use city::City;
pub use parse::AddressMatch;
pub use prefecture::Prefecture;

/// 住所検索の統一エントリポイント。
///
/// 都道府県・市区町村・郵便番号住所のすべての検索をこの型に集約している。
pub struct AddressSearch;

impl AddressSearch {
    /// すべての埋め込みデータと検索インデックスを事前に構築する。
    ///
    /// 呼ばなくても各データは初回アクセス時に自動で構築されるが、
    /// アプリケーションの起動時に呼んでおくと初回検索の遅延を避けられる。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// AddressSearch::preload();
    /// assert!(!AddressSearch::find_by_zipcode("100-0001").is_empty());
    /// ```
    pub fn preload() {
        let _ = prefecture::all();
        let _ = city::all();
        address::preload();
    }

    /// 郵便番号で住所を検索する。数字以外の文字は無視するため、
    /// `1000001` と `100-0001` はどちらも同じ結果を返す。
    /// 数字がちょうど7桁でない入力は空を返す。
    /// 1つの郵便番号が複数の町域に対応する場合があるため複数件返ることがある。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// let addresses = AddressSearch::find_by_zipcode("100-0001");
    /// assert_eq!(addresses[0].prefecture_name, "東京都");
    /// assert_eq!(addresses[0].city_name, "千代田区");
    /// assert_eq!(addresses[0].town, "千代田");
    ///
    /// // 数字が7桁でない入力は空を返す
    /// assert!(AddressSearch::find_by_zipcode("100-001").is_empty());
    /// ```
    pub fn find_by_zipcode(zipcode: &str) -> &'static [&'static Address] {
        address::find_by_zipcode(zipcode)
    }

    /// 全住所データを市区町村コード順で返す。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// assert!(AddressSearch::addresses().len() > 100_000);
    /// ```
    pub fn addresses() -> &'static [Address] {
        address::all()
    }

    /// 住所文字列から都道府県・市区町村・町域(郵便番号)を特定する。
    ///
    /// 先頭から「都道府県名(省略可)→市区町村名(郡名の有無は不問)→町域名」の順に
    /// 最長前方一致で解析し、特定できた階層までを[`AddressMatch`]で返す。
    /// 空白・全角数字・「大字」・ヶ/ケ/がの表記ゆれは吸収する。
    /// 同名の町域が丁目で複数の郵便番号に分かれる場合(札幌市中央区「大通西」など)は、
    /// 町域に続く数字を丁目とみなして[`Address::town_note`]の範囲と照合し特定する。
    /// 番地は解析しない。
    /// 町域が一覧にない場合は「以下に掲載がない場合」の郵便番号を返す。
    /// 同名の市区町村は[`AddressSearch::find_city_by_name`]と同様にコード最小を優先する。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// let matched = AddressSearch::parse_address("東京都世田谷区太子堂4丁目1-1");
    /// assert_eq!(matched.prefecture.unwrap().name, "東京都");
    /// assert_eq!(matched.city.unwrap().name, "世田谷区");
    /// assert_eq!(matched.address.unwrap().zip, "1540004");
    ///
    /// // 都道府県の省略、郡名付きの表記にも対応
    /// let matched = AddressSearch::parse_address("夕張郡長沼町木詰");
    /// assert_eq!(matched.prefecture.unwrap().name, "北海道");
    /// assert_eq!(matched.address.unwrap().zip, "0691329");
    /// ```
    pub fn parse_address(address: &str) -> AddressMatch {
        parse::parse_address(address)
    }

    /// 市区町村コードで住所を検索する。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// let addresses = AddressSearch::find_addresses_by_city_id(13101);
    /// assert!(!addresses.is_empty());
    /// assert!(addresses.iter().all(|address| address.city_name == "千代田区"));
    /// ```
    pub fn find_addresses_by_city_id(city_id: i32) -> &'static [Address] {
        address::find_by_city_id(city_id)
    }

    /// 全47都道府県をコード順で返す。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// assert_eq!(AddressSearch::prefectures().len(), 47);
    /// assert_eq!(AddressSearch::prefectures()[0].name, "北海道");
    /// ```
    pub fn prefectures() -> &'static [Prefecture] {
        prefecture::all()
    }

    /// 都道府県コードで都道府県を検索する。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// let tokyo = AddressSearch::find_prefecture_by_id(13).unwrap();
    /// assert_eq!(tokyo.name, "東京都");
    /// assert_eq!(tokyo.en_name, "tokyo");
    /// assert!(AddressSearch::find_prefecture_by_id(99).is_none());
    /// ```
    pub fn find_prefecture_by_id(id: i32) -> Option<&'static Prefecture> {
        prefecture::find_by_id(id)
    }

    /// 名称で都道府県を検索する。短縮名ではなく正式名称(例: `東京都`)に一致する。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// let osaka = AddressSearch::find_prefecture_by_name("大阪府").unwrap();
    /// assert_eq!(osaka.id, 27);
    /// assert!(AddressSearch::find_prefecture_by_name("大阪").is_none());
    /// ```
    pub fn find_prefecture_by_name(name: &str) -> Option<&'static Prefecture> {
        prefecture::find_by_name(name)
    }

    /// 全市区町村をコード順で返す。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// assert!(AddressSearch::cities().len() > 1900);
    /// ```
    pub fn cities() -> &'static [City] {
        city::all()
    }

    /// 市区町村コードで市区町村を検索する。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// let city = AddressSearch::find_city_by_id(13101).unwrap();
    /// assert_eq!(city.name, "千代田区");
    /// assert!(AddressSearch::find_city_by_id(99999).is_none());
    /// ```
    pub fn find_city_by_id(id: i32) -> Option<&'static City> {
        city::find_by_id(id)
    }

    /// 名称(例: `札幌市中央区`)で市区町村を検索する。
    /// 同名の市区町村が複数の都道府県に存在する場合(府中市・伊達市など)は、
    /// 市区町村コードが最小のものを返す。都道府県を指定して特定する場合は
    /// [`AddressSearch::find_cities_by_prefecture_id`]の結果から名称で絞り込む。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// let sakai = AddressSearch::find_city_by_name("堺市堺区").unwrap();
    /// assert_eq!(sakai.id, 27141);
    /// // 政令指定都市の行政区は親となる市のコードを持つ
    /// assert_eq!(sakai.designated_city_id, 27140);
    ///
    /// // 府中市は東京都(13206)と広島県(34208)に存在し、コード最小の東京都が返る
    /// let fuchu = AddressSearch::find_city_by_name("府中市").unwrap();
    /// assert_eq!(fuchu.id, 13206);
    /// ```
    pub fn find_city_by_name(name: &str) -> Option<&'static City> {
        city::find_by_name(name)
    }

    /// 都道府県コードに属する市区町村を返す。
    ///
    /// # Examples
    ///
    /// ```
    /// use jp_address_search::AddressSearch;
    ///
    /// let cities = AddressSearch::find_cities_by_prefecture_id(13);
    /// assert!(cities.iter().any(|city| city.name == "千代田区"));
    /// assert!(cities.iter().all(|city| city.prefecture_id == 13));
    /// ```
    pub fn find_cities_by_prefecture_id(prefecture_id: i32) -> &'static [City] {
        city::find_by_prefecture_id(prefecture_id)
    }
}
