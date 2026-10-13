use serde_json::{json,Value};
fn main() {
    let base:Value=serde_json::from_str(include_str!("../tests/fixtures/synthetic-300-months.json")).unwrap();
    let corrected:Value=serde_json::from_str(include_str!("../tests/fixtures/synthetic-corrected.json")).unwrap();
    let a=previus_rgps::calculate_json(base).unwrap();
    let b=previus_rgps::calculate_json(corrected).unwrap();
    let blocked=previus_rgps::calculate_json(json!({})).unwrap();
    println!("{}",serde_json::to_string_pretty(&json!({"fixtures":"synthetic_only","plain":{"status":a["status"],"resumo":a["data"]["native_result"]["resumo"]},"corrected":{"status":b["status"],"resumo":b["data"]["native_result"]["resumo"],"correction":b["data"]["native_result"]["correcao"]},"missing_input":{"status":blocked["status"],"data":blocked["data"],"missing":blocked["missing"]},"capabilities":previus_rgps::capabilities()})).unwrap());
}
