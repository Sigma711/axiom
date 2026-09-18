use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct BinanceKline(
    i64,
    String,
    String,
    String,
    String,
    String,
);

fn main() {
    let json = r#"[[1774026000000,"69829.27","70039.78","69388.00","70000.00","998.84797",1774029599999,"69631317.23319380",297124,"513.14661000","35779801.14582250","0"]]"#;
    let result: Result<Vec<BinanceKline>, _> = serde_json::from_str(json);
    println!("{:?}", result);
}