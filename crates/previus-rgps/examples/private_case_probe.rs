//! Explicit local diagnostics. Sensitive output is written only to the chosen private directory.
use previus_workspace::Calculation;
use serde_json::{Value,json};
use std::{path::PathBuf,io::Write};
fn write_new(path:PathBuf,bytes:&[u8]){let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).expect("Create private artifact without overwriting");file.write_all(bytes).expect("Write private artifact");}
fn main(){
    let args:Vec<_>=std::env::args_os().skip(1).collect();if args.len()!=3{eprintln!("Usage: private_case_probe <local-CNIS.pdf> <private-parameters.json> <new-private-output-directory>");std::process::exit(2)}
    let source=PathBuf::from(&args[0]);let parameters:Value=serde_json::from_slice(&std::fs::read(&args[1]).expect("Explicit private parameters")).expect("Private parameters JSON");let output=PathBuf::from(&args[2]);
    assert!(!output.exists(),"Use a new output directory; no old evidence overwritten");std::fs::create_dir_all(&output).expect("Private output directory");
    let before=std::fs::read(&source).expect("Authorized local source");let cnis=previus_cnis::import_pdf(&source).expect("Local CNIS parser");
    let cutoff=parameters["last_salary_month"].as_str().expect("Explicit last salary month");
    let eligible:Vec<_>=cnis.salary_months.keys().filter(|m|m.as_str()>="1994-07"&&m.as_str()<=cutoff).cloned().collect();
    let mut case=Calculation{kind:"RGPS".into(),name:"Conferência privada do CNIS - sem resultado presumido".into(),identity:cnis.identity.clone(),cnis:Some(cnis.clone()),sex:parameters["sex"].as_str().map(str::to_string),disability:parameters["disability"].as_str().map(str::to_string),dib:parameters["dib"].as_str().expect("Explicit DIB").into(),..Default::default()};
    case.options.rule_policy=parameters["policy"].to_string();
    let outcome=previus_rgps::calculate_case(&case).expect("Adapter returns actual blockers, not fabricated RMI");
    let summary=json!({"native_import":cnis.summary(),"pbc_months_present_to_cutoff":eligible.len(),"cutoff":cutoff,"adapter_status":outcome["status"],"financial_result_generated":!outcome["data"].is_null(),"original_unchanged":before==std::fs::read(&source).unwrap(),"scope":"Unreviewed import diagnostics only; no entitlement or RMI validation"});
    write_new(output.join("input.private.json"),&serde_json::to_vec_pretty(&case).unwrap());write_new(output.join("outcome.private.json"),&serde_json::to_vec_pretty(&outcome).unwrap());write_new(output.join("summary.private.json"),&serde_json::to_vec_pretty(&summary).unwrap());
    println!("Local private probe completed. Original preserved: {}. Native status: {}. No client data printed.",summary["original_unchanged"],outcome["status"]);
}
