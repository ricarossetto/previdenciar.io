//! Generate two actual PDFs with fictional contribution-review data only.
use previus_reports::{render_pdf, ReportKind, ReportOptions};
use previus_workspace::{Calculation,Workspace};
use previus_workspace::contributions::{ContributionRecord,ContributionReview,ContributionChange};
use std::io::Write;
fn main(){
    let output=std::env::args().nth(1).map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from("target/contribution-example"));
    std::fs::create_dir_all(&output).expect("Create example directory");
    let cnis=previus_cnis::import_text("CNIS\nCompetência Remuneração Indicadores\n01/2020 100,10\n01/2020 200,20\n02/2020 300,30").expect("Synthetic input");
    let mut records:Vec<_>=cnis.salary_entries.iter().map(|e|ContributionRecord{key:format!("source:{}",e.source.index),source_indices:vec![e.source.index],competence:e.competence.clone(),amount:format!("{}.{:02}",e.amount_centavos/100,e.amount_centavos%100),currency:"unconfirmed".into(),link_sequence:e.link_sequence,indicators:e.indicators.join(", "),confirmed:false,excluded:false,evidence:String::new()}).collect();
    let before=records[0].clone();records[0].amount="110.11".into();records[0].currency="BRL".into();records[0].evidence="Documento demonstrativo fictício: retificação nominal, sem finalidade jurídica".into();
    let change=ContributionChange{sequence:1,action:"edit".into(),reason:records[0].evidence.clone(),at:"2026-10-13T10:00:00Z".into(),before:vec![before],after:vec![records[0].clone()]};
    let review=ContributionReview{version:1,source_sha256:cnis.metadata.source_sha256.clone(),records,history:vec![change],applied:false};
    let mut case=Calculation{cnis:Some(cnis),contribution_review:Some(review),name:"DEMONSTRAÇÃO SINTÉTICA - NÃO É CÁLCULO DE BENEFÍCIO".into(),..Default::default()};
    case.identity.full_name=Some("PESSOA FICTÍCIA - DADOS DE DEMONSTRAÇÃO".into());
    for (personal,name) in [(false,"Livro-Contribuicoes-Privado.pdf"),(true,"Livro-Contribuicoes-Demonstrativo.pdf")]{
        let bytes=render_pdf(&case,&Workspace::default(),ReportOptions{kind:ReportKind::Salaries,include_personal:personal},None).expect("Real Rust PDF renderer");
        assert!(bytes.starts_with(b"%PDF-"));let doc=lopdf::Document::load_mem(&bytes).expect("Readable PDF");
        let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(output.join(name)).expect("Create a new file; never overwrite");file.write_all(&bytes).expect("Write example PDF");
        println!("{}",serde_json::json!({"file":name,"pages":doc.get_pages().len(),"bytes":bytes.len(),"synthetic":true,"benefit_calculated":false,"personal_mode":personal}));
    }
}
