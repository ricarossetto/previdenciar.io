//! Local arithmetic benchmark only. Never reports eligibility or a benefit RMI.
use std::{path::PathBuf,io::Write};
use serde_json::{Value,json};
use rgps_core::{calendar::Competencia,money::parse_money,rmi::calcular_pbc_com_correcao};
fn main(){
    let args:Vec<_>=std::env::args_os().skip(1).collect();if args.len()!=2{eprintln!("Usage: local_pbc_probe <private-wages.json> <new-private-output.json>");std::process::exit(2)}
    let input:Value=serde_json::from_slice(&std::fs::read(&args[0]).expect("Explicit private input")).expect("Private JSON");let output=PathBuf::from(&args[1]);assert!(!output.exists(),"Never overwrite benchmark evidence");
    let reference=Competencia::parse(input["reference"].as_str().expect("Explicit reference month"),"referência").expect("Reference");let discard=input["discard"].as_bool().expect("Explicit discard selection");
    let rows=input["wages"].as_array().expect("Explicit wage list");assert!(!rows.is_empty()&&rows.len()<=10000,"Bounded nonempty private input");
    let wages:Vec<_>=rows.iter().map(|row|(Competencia::parse(row["month"].as_str().expect("Month"),"competência").expect("Valid month"),parse_money(row["amount"].as_str().expect("Canonical amount"),"valor nominal").expect("Exact amount"))).collect();
    let diagnostic=match calcular_pbc_com_correcao(&wages,&reference,discard){Ok(pbc)=>json!({"status":"arithmetic_diagnostic","eligibility_calculated":false,"benefit_rmi_calculated":false,"source_records_reviewed":false,"snapshot_source":"existing internal RGPS tables, not auto-updated or normative certification","pbc":pbc}),Err(error)=>json!({"status":"blocked","benefit_rmi_calculated":false,"message":error.to_string()})};
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(output).expect("Private write-once artifact");file.write_all(&serde_json::to_vec_pretty(&diagnostic).unwrap()).expect("Write private diagnostic");println!("Private PBC arithmetic diagnostic written. No personal input or financial figure printed.");
}
