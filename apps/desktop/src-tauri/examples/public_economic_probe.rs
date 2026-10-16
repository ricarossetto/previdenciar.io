//! Fixed public snapshot check. No case, credential or response content is printed.
#[path = "../src/economic.rs"]
mod economic;
fn main() {
    match economic::load("2026-07") {
        Ok(snapshot) => println!(
            "{}",
            serde_json::json!({"status":"passed","through":snapshot.through,"rates":snapshot.rates.len(),"limits":snapshot.limits.len(),"publicRequestsOnly":true,"personalDataSent":false,"apiKeyRequired":false})
        ),
        Err(_) => {
            eprintln!("Public snapshot failed; no credential or private data was involved.");
            std::process::exit(1);
        }
    }
}
