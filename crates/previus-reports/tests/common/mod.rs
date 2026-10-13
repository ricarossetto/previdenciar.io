#![allow(dead_code)]
use previus_cnis::*;
use previus_workspace::{AssistantInterview, Calculation, Workspace};
use std::collections::BTreeMap;
pub fn source(index:usize)->SourceRef{SourceRef{page:(index/40+1)as u32,line:index%40+1,index,byte_offset:index*100}}
pub fn synthetic_case()->(Calculation,Workspace){
    let identity=Identity{full_name:Some("IDENTIDADETESTE João da Conceição".into()),cpf:Some("CPFMARCADOR-12345678900".into()),nit:Some("NITMARCADOR-987654321".into()),birth_date:Some("1970-02-10".into()),mother_name:Some("MÃEMARCADOR Ana Teste".into())};
    let mut cnis=ImportResult{metadata:ImportMetadata{extractor:"fixture-sintetica".into(),source_sha256:Some("a".repeat(64)),page_count:15,parsed_pages:15,legal_validation_performed:false},identity:identity.clone(),salary_entries:vec![],salary_months:BTreeMap::new(),links:vec![],benefits:vec![],findings:vec![],warnings:vec![]};
    for i in 0..14 {cnis.links.push(EmploymentLink{sequence:i+1,raw_employer:Some(format!("EMPREGADORMARCADOR Empresa {i} Teste")),affiliation:Some("empregado".into()),start_date:Some(format!("{}-01-01",1988+i)),end_date:if i<5{None}else{Some(format!("{}-12-31",1989+i))},reported_last_salary_month:Some(format!("{}-12",1989+i)),inferred_last_salary_end:None,indicators:if i<6{vec!["PRPPS".into()]}else{vec![]},prpps:i<6,validation_status:if i<6{ValidationStatus::UnresolvedPrpps}else{ValidationStatus::Unvalidated},excluded_from_validated_domain:i<6,source:source(i as usize)});}
    for i in 0..560 {let month_index=if i<425{i}else{i-425};let comp=format!("{}-{:02}",1988+month_index/12,month_index%12+1);let excluded=i%14<6;let entry=SalaryEntry{competence:comp.clone(),amount_centavos:200000+i as i64,indicators:if i%15==0{vec!["PREC-MENOR-MIN".into(),"INDICADORPESSOALMARCADOR".into()]}else{vec![]},source:source(i),link_sequence:Some((i%14+1)as u32),amount_kind:AmountKind::Remuneration,validation_status:if excluded{ValidationStatus::UnresolvedPrpps}else{ValidationStatus::Unvalidated},excluded_from_validated_domain:excluded};let month=cnis.salary_months.entry(comp).or_default();month.count+=1;month.amount_sum_centavos+=entry.amount_centavos;month.entry_indices.push(i);month.contains_unresolved_prpps|=excluded;cnis.salary_entries.push(entry);}
    cnis.benefits.push(BenefitRecord{sequence:15,species:Some(41),status:BenefitStatus::Denied,included_as_employment_link:false,source:source(560)});
    cnis.warnings.push(Finding{code:"currency_units_unverified".into(),severity:Severity::Warning,message:"AVISOFONTEMARCADOR Conferir unidades monetárias históricas".into(),count:560,source:None});
    let assistant=AssistantInterview{mode:"assisted".into(),current_benefit_received:None,import_requested:Some(true),process_kind:Some("Simulação do benefício".into()),goal:Some("Calcular um novo benefício".into()),review_confirmed:false,completed_steps:vec!["intro".into(),"import".into()]};
    let case=Calculation{id:12,name:"NOMEMARCADOR Teste de conferência".into(),kind:"rgps".into(),identity,cnis:Some(cnis),assistant:Some(assistant),dib:"2026-10-10".into(),notes:"NOTAMARCADOR Nota privada do operador".into(),..Default::default()};
    let mut ws=Workspace::default();ws.settings.office_name="ESCRITORIOMARCADOR Escritório teste".into();ws.settings.report_footer="RODAPEMARCADOR Conteúdo privado".into();ws.calculations.push(case.clone());(case,ws)
}
pub fn calculated_case()->(Calculation,Workspace,serde_json::Value){
    let mut case:Calculation=serde_json::from_str(include_str!("../../../previus-rgps/tests/fixtures/calculation-synthetic.json")).unwrap();
    case.assistant=Some(AssistantInterview{mode:"assisted".into(),current_benefit_received:Some(false),import_requested:Some(false),process_kind:Some("Simulação do benefício".into()),goal:Some("Calcular um novo benefício".into()),review_confirmed:true,completed_steps:vec!["intro".into(),"benefit".into(),"import".into(),"process".into(),"goal".into(),"parameters".into(),"periods".into(),"salaries".into(),"policy".into(),"review".into()]});
    let outcome=previus_rgps::calculate_case(&case).unwrap();assert_eq!(outcome["status"],"experimental");let mut ws=Workspace::default();ws.calculations.push(case.clone());(case,ws,outcome)
}
pub fn extract(bytes:&[u8])->(lopdf::Document,String){
    let doc=lopdf::Document::load_mem(bytes).expect("actual PDF parse");
    let pages:Vec<u32>=doc.get_pages().keys().copied().collect();
    let text=doc.extract_text(&pages).expect("ToUnicode extraction");
    let normalized=text.split_whitespace().collect::<Vec<_>>().join(" ");
    (doc,normalized)
}
pub fn compact(s:&str)->String{s.chars().filter(|c|!c.is_whitespace()).collect()}
