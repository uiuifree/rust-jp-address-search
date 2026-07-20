//! 検索速度の計測。実行: `cargo run --release --example bench`

use jp_address_search::AddressSearch;
use std::hint::black_box;
use std::time::Instant;

fn bench(name: &str, iterations: u32, mut target: impl FnMut()) {
    let start = Instant::now();
    for _ in 0..iterations {
        target();
    }
    println!("{name}: {:?}", start.elapsed() / iterations);
}

fn main() {
    let start = Instant::now();
    AddressSearch::preload();
    println!("preload(初回ロード): {:?}", start.elapsed());

    bench("find_by_zipcode", 1_000_000, || {
        black_box(AddressSearch::find_by_zipcode(black_box("060-0041")));
    });
    bench("find_addresses_by_city_id", 1_000_000, || {
        black_box(AddressSearch::find_addresses_by_city_id(black_box(13101)));
    });
    bench("find_prefecture_by_id", 1_000_000, || {
        black_box(AddressSearch::find_prefecture_by_id(black_box(13)));
    });
    bench("find_city_by_id", 1_000_000, || {
        black_box(AddressSearch::find_city_by_id(black_box(13101)));
    });
    bench("find_cities_by_prefecture_id", 1_000_000, || {
        black_box(AddressSearch::find_cities_by_prefecture_id(black_box(13)));
    });
    bench("parse_address(都道府県あり)", 100_000, || {
        black_box(AddressSearch::parse_address(black_box(
            "東京都世田谷区太子堂4丁目1-1",
        )));
    });
    bench("parse_address(都道府県なし)", 100_000, || {
        black_box(AddressSearch::parse_address(black_box(
            "札幌市中央区大通西25丁目",
        )));
    });
}
