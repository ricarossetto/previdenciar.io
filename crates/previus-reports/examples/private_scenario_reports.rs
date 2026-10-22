//! Explicit, private verification runner. Never embeds a client's data in source or fixtures.
use previus_reports::{render_pdf,ReportOptions,ReportKind};
use previus_workspace::{Calculation,Workspace};
use std::{path::PathBuf,io::Write};
fn write_new(path:PathBuf,bytes:&[u8]){let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).expect("Write a new private file only");file.write_all(bytes).expect("Write private result");}
fn main(){let args:Vec<_>=std::env::args_os().skip(1).collect();if args.len()<2||args.len()>3{eprintln!("Usage: private_scenario_reports <explicit-private-case.json> <new-private-output-dir> [--include-personal]");std::process::exit(2)}
 let personal=args.get(2).is_some_and(|v|v=="--include-personal");if args.len()==3&&!personal{panic!("Unknown option")}
 let case:Calculation=serde_json::from_slice(&std::fs::read(&args[0]).expect("Authorized local case")).expect("Typed private case");
 let output=PathBuf::from(&args[1]);assert!(!output.exists(),"Use a new private output directory; never overwrite evidence");std::fs::create_dir_all(&output).unwrap();
 let original=serde_json::to_value(&case).unwrap();let outcome=previus_rgps::calculate_case(&case).expect("Actual independent calculation");
 write_new(output.join("outcome.private.json"),&serde_json::to_vec_pretty(&outcome).unwrap());
 let options=[(ReportKind::Memory,"Memoria-RMI.pdf"),(ReportKind::Periods,"Tempo-Contribuicao.pdf"),(ReportKind::Salaries,"Salarios-Contribuicao.pdf"),(ReportKind::Summary,"Resumo.pdf"),(ReportKind::Detailed,"Detalhado.pdf"),(ReportKind::Transitions,"Comparacao-Regras.pdf"),(ReportKind::Indicators,"Pendencias.pdf")];
 let mut manifest=vec![];for(kind,name)in options{let bytes=render_pdf(&case,&Workspace::default(),ReportOptions::new(kind, personal),Some(&outcome)).expect("Real native PDF renderer");let doc=lopdf::Document::load_mem(&bytes).expect("Readable PDF");manifest.push(serde_json::json!({"file":name,"pages":doc.get_pages().len(),"bytes":bytes.len(),"identityIncluded":personal}));write_new(output.join(name),&bytes);}
 assert_eq!(original,serde_json::to_value(&case).unwrap(),"Original case never mutated");write_new(output.join("manifest.private.json"),&serde_json::to_vec_pretty(&manifest).unwrap());println!("Private scenario calculated; native PDFs written; source case preserved. No identity or financial values printed.");
}
