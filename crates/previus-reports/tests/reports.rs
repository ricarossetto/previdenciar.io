mod common;
use common::*;
use previus_cnis::{AmountKind, EmploymentLink, ImportResult, SalaryEntry, SourceRef, ValidationStatus};
use previus_reports::{render_pdf,ReportKind,ReportOptions};
use previus_workspace::{Calculation,Workspace};
use serde_json::json;
fn opts(kind:ReportKind,personal:bool)->ReportOptions{ReportOptions::new(kind, personal)}
#[test]fn public_kinds_parse_and_copy(){for(name,kind)in[("detailed",ReportKind::Detailed),("summary",ReportKind::Summary),("periods",ReportKind::Periods),("salaries",ReportKind::Salaries),("memory",ReportKind::Memory),("transitions",ReportKind::Transitions),("indicators",ReportKind::Indicators),("quality",ReportKind::Quality)]{assert_eq!(ReportKind::parse(name).unwrap(),kind);let copy=kind;assert_eq!(copy,kind);}for bad in ["","PDF","Detailed","detailed ","../summary"]{assert!(ReportKind::parse(bad).is_err());}}
#[test]fn quality_report_apurates_grace_and_prorogation_by_link(){
    let(mut case,ws)=synthetic_case();
    let mut entries=Vec::new();
    for index in 0..130u32{
        let year=2010+index/12;let month=index%12+1;
        entries.push(SalaryEntry{competence:format!("{year}-{month:02}"),amount_centavos:100000,indicators:vec![],link_sequence:Some(1),source:SourceRef{page:1,line:index as usize+1,index:index as usize,byte_offset:0},amount_kind:AmountKind::Remuneration,validation_status:ValidationStatus::Unvalidated,excluded_from_validated_domain:false});
    }
    let cnis=ImportResult{metadata:previus_cnis::ImportMetadata{extractor:"synthetic".into(),source_sha256:Some("a".repeat(64)),page_count:1,parsed_pages:1,legal_validation_performed:false},identity:case.identity.clone(),salary_entries:entries,salary_months:Default::default(),
        links:vec![EmploymentLink{sequence:1,raw_employer:Some("EMPREGADOR FICTÍCIO".into()),affiliation:Some("Empregado".into()),start_date:Some("2010-01-01".into()),end_date:None,reported_last_salary_month:None,inferred_last_salary_end:None,indicators:vec![],prpps:false,validation_status:ValidationStatus::Unvalidated,excluded_from_validated_domain:false,source:SourceRef{page:1,line:1,index:0,byte_offset:0}}],
        benefits:vec![],findings:vec![],warnings:vec![]};
    case.cnis=Some(cnis);
    let pdf=render_pdf(&case,&ws,opts(ReportKind::Quality,false),None).unwrap();
    let(_,text)=extract(&pdf);
    assert!(text.contains("Cálculo da qualidade de segurado por vínculo"));
    assert!(text.contains("Vínculos e contribuições apuradas"));
    assert!(text.contains("Período de graça por vínculo"));
    assert!(text.contains("Prorrogada: mais de 120 contribuições"));
    assert!(text.contains("Meses contados"));
    assert!(text.contains("art. 15"));
    assert!(text.contains("Não aplicável")||text.contains("Prorrogada"));
    assert!(!text.contains("EMPREGADOR FICTÍCIO"),"private mode must hide the employer");
    let personal=extract(&render_pdf(&case,&ws,opts(ReportKind::Quality,true),None).unwrap()).1;
    assert!(personal.contains("EMPREGADOR FICTÍCIO"));
}
#[test]fn quality_report_shows_the_computed_grace_dates(){
    let(mut case,ws)=synthetic_case();
    let mut entries=Vec::new();
    for index in 0..130u32{
        let year=2010+index/12;let month=index%12+1;
        entries.push(SalaryEntry{competence:format!("{year}-{month:02}"),amount_centavos:100000,indicators:vec![],link_sequence:Some(1),source:SourceRef{page:1,line:index as usize+1,index:index as usize,byte_offset:0},amount_kind:AmountKind::Remuneration,validation_status:ValidationStatus::Unvalidated,excluded_from_validated_domain:false});
    }
    let cnis=ImportResult{metadata:previus_cnis::ImportMetadata{extractor:"synthetic".into(),source_sha256:Some("a".repeat(64)),page_count:1,parsed_pages:1,legal_validation_performed:false},identity:case.identity.clone(),salary_entries:entries,salary_months:Default::default(),
        links:vec![EmploymentLink{sequence:1,raw_employer:Some("EMPREGADOR FICTÍCIO".into()),affiliation:Some("Empregado".into()),start_date:Some("2010-01-01".into()),end_date:None,reported_last_salary_month:None,inferred_last_salary_end:None,indicators:vec![],prpps:false,validation_status:ValidationStatus::Unvalidated,excluded_from_validated_domain:false,source:SourceRef{page:1,line:1,index:0,byte_offset:0}}],
        benefits:vec![],findings:vec![],warnings:vec![]};
    case.cnis=Some(cnis);
    let pdf=render_pdf(&case,&ws,opts(ReportKind::Quality,false),None).unwrap();
    let(_,text)=extract(&pdf);
    assert!(text.contains("15/10/2021"),"base grace date missing: {:?}", text.split("Graça base").nth(1).unwrap_or("absent").chars().take(160).collect::<String>());
    assert!(text.contains("15/10/2022"),"prorogated grace date missing");
}
#[test]fn quality_report_does_not_count_a_month_twice_and_excludes_benefit_time(){
    let(mut case,ws)=synthetic_case();
    let entry=|competence:&str,sequence:u32,index:usize|SalaryEntry{competence:competence.into(),amount_centavos:100000,indicators:vec![],link_sequence:Some(sequence),source:SourceRef{page:1,line:index+1,index,byte_offset:0},amount_kind:AmountKind::Remuneration,validation_status:ValidationStatus::Unvalidated,excluded_from_validated_domain:false};
    let link=|sequence:u32,affiliation:&str,start:&str,end:Option<&str>|EmploymentLink{sequence,raw_employer:Some("EMPREGADOR FICTÍCIO".into()),affiliation:Some(affiliation.into()),start_date:Some(start.into()),end_date:end.map(str::to_string),reported_last_salary_month:None,inferred_last_salary_end:None,indicators:vec![],prpps:false,validation_status:ValidationStatus::Unvalidated,excluded_from_validated_domain:false,source:SourceRef{page:1,line:1,index:0,byte_offset:0}};
    let cnis=ImportResult{metadata:previus_cnis::ImportMetadata{extractor:"synthetic".into(),source_sha256:Some("b".repeat(64)),page_count:1,parsed_pages:1,legal_validation_performed:false},identity:case.identity.clone(),
        salary_entries:vec![entry("2010-01",1,0),entry("2010-02",1,1),entry("2010-02",2,2),entry("2010-03",2,3),entry("2010-04",3,4)],
        salary_months:Default::default(),
        links:vec![link(1,"Empregado","2010-01-01",None),link(2,"Empregado","2010-02-01",None),link(3,"Em Benefício","2010-04-01",None)],
        benefits:vec![],findings:vec![],warnings:vec![]};
    case.cnis=Some(cnis);
    let pdf=render_pdf(&case,&ws,opts(ReportKind::Quality,false),None).unwrap();
    let(_,text)=extract(&pdf);
    assert!(text.contains("2010-02 (competência já contada em período anterior)"),"overlap must be declared: {text}");
    assert!(text.contains("2010-04 (período de gozo de benefício)"),"benefit time must be excluded: {text}");
    assert!(text.contains("Meses não contados por competência em outro vínculo"));
    assert!(text.contains("Meses não contados por período de gozo de benefício"));
    assert!(text.contains("art. 19-C"),"the regulation must be cited: {text}");
}
#[test]fn quality_report_without_a_source_states_that_nothing_was_apurated(){    let case=Calculation::default();
    let pdf=render_pdf(&case,&Workspace::default(),opts(ReportKind::Quality,false),None).unwrap();
    let(_,text)=extract(&pdf);
    assert!(text.contains("Sem CNIS anexado"));
    assert!(text.contains("não há vínculos de origem para apurar"));
}
#[test]fn every_kind_is_an_actual_readable_a4_pdf(){for kind in [ReportKind::Detailed,ReportKind::Summary,ReportKind::Periods,ReportKind::Salaries,ReportKind::Memory,ReportKind::Transitions,ReportKind::Indicators,ReportKind::Quality,ReportKind::Quality]{let bytes=render_pdf(&Calculation::default(),&Workspace::default(),opts(kind,false),None).unwrap();assert!(bytes.starts_with(b"%PDF-"));assert!(bytes.len()>1000);let(doc,text)=extract(&bytes);assert!(!doc.get_pages().is_empty());assert!(text.contains("DADOS PENDENTES"));for id in doc.get_pages().values(){let page=doc.get_object(*id).unwrap().as_dict().unwrap();let rect=page.get(b"MediaBox").unwrap().as_array().unwrap();assert!((rect[2].as_float().unwrap()-595.276).abs()<0.1);assert!((rect[3].as_float().unwrap()-841.89).abs()<0.1);}}}
#[test]fn detailed_keeps_425_months_560_entries_and_14_links(){let(case,ws)=synthetic_case();let pdf=render_pdf(&case,&ws,opts(ReportKind::Detailed,false),None).unwrap();let(doc,text)=extract(&pdf);assert!(doc.get_pages().len()>10);let flat=compact(&text);for value in ["425","560","14","PRPPS","Indeferido","2.005,59","05/2023","560"]{assert!(flat.contains(&compact(value)),"missing {value}");}assert!(text.contains("Todos os lançamentos importados"));assert!(text.matches("Competência").count()>20,"repeated table headers");assert!(text.contains(&format!("Página {}",doc.get_pages().len())));}
#[test]fn no_arbitrary_saved_result_or_blocked_rmi(){let(mut case,ws)=synthetic_case();case.result=Some(json!({"status":"experimental","data":{"native_result":{"resumo":{"rmi":"987654.32"}}}}));let blocked=json!({"status":"blocked","data":{"native_result":{"resumo":{"rmi":"123456.78"}}},"blockers":[{"code":"source_review_required","message":"DADOSECRETO"}]});for outcome in [None,Some(&blocked)]{let pdf=render_pdf(&case,&ws,opts(ReportKind::Detailed,false),outcome).unwrap();let(_,text)=extract(&pdf);assert!(text.contains("DADOS PENDENTES"));for val in ["987.654,32","123.456,78","DADOSECRETO"]{assert!(!text.contains(val));}assert!(!text.contains("RMI experimental"));}}
#[test]fn private_mode_excludes_identifiers_and_arbitrary_fields(){let(case,ws)=synthetic_case();let (_,_,mut positive)=calculated_case();positive["warnings"]=json!(["AVISORESULTADOMARCADOR with a personal name"]);positive["data"]["native_result"]["unknown_personal"]=json!({"name":"FACETAMARCADOR","cpf":"12345678900"});positive["data"]["native_result"]["parametros"]["PRIVATEFIELDKEYMARCADOR"]=json!("PRIVATEPARAMMARCADOR");let pdf=render_pdf(&case,&ws,opts(ReportKind::Detailed,false),Some(&positive)).unwrap();let(_,text)=extract(&pdf);let flat=compact(&text);for secret in ["IDENTIDADETESTE","CPFMARCADOR","NITMARCADOR","MÃEMARCADOR","EMPREGADORMARCADOR","INDICADORPESSOALMARCADOR","AVISOFONTEMARCADOR","NOMEMARCADOR","NOTAMARCADOR","ESCRITORIOMARCADOR","RODAPEMARCADOR","AVISORESULTADOMARCADOR","FACETAMARCADOR","PRIVATEPARAMMARCADOR","PRIVATEFIELDKEYMARCADOR"]{assert!(!flat.contains(&compact(secret)),"private field present: {secret}");}assert!(text.contains("01/1988"));assert!(text.contains("1.600,00"));}
#[test]fn include_personal_shows_authorized_identification_and_accents(){let(case,ws)=synthetic_case();let pdf=render_pdf(&case,&ws,opts(ReportKind::Detailed,true),None).unwrap();let(_,text)=extract(&pdf);let flat=compact(&text);for value in ["IDENTIDADETESTE","João da Conceição","CPFMARCADOR","NITMARCADOR","EMPREGADORMARCADOR","ESCRITORIOMARCADOR","RODAPEMARCADOR","NOTAMARCADOR"]{assert!(flat.contains(&compact(value)),"missing authorized field {value}");}}
#[test]fn source_case_and_workspace_are_immutable(){let(case,ws)=synthetic_case();let before=serde_json::to_vec(&case).unwrap();let ws_before=serde_json::to_vec(&ws).unwrap();render_pdf(&case,&ws,opts(ReportKind::Detailed,false),None).unwrap();assert_eq!(before,serde_json::to_vec(&case).unwrap());assert_eq!(ws_before,serde_json::to_vec(&ws).unwrap());}
#[test]fn actual_rgps_outcome_renders_summary_and_memory(){let(case,ws,o)=calculated_case();assert_eq!(o["data"]["native_result"]["resumo"]["rmi"],"1600.00");for kind in [ReportKind::Detailed,ReportKind::Summary,ReportKind::Memory,ReportKind::Transitions]{let pdf=render_pdf(&case,&ws,opts(kind,false),Some(&o)).unwrap();let(_,text)=extract(&pdf);assert!(text.contains("EXPERIMENTAL"));assert!(text.contains("1.600,00"));if kind==ReportKind::Memory||kind==ReportKind::Detailed{assert!(text.contains("2.000,00"));assert!(text.contains("80%"));assert!(text.contains("Todas as etapas numéricas"));}}}
#[test]fn corrected_outcome_has_all_native_parcels(){let(case,ws,_)=calculated_case();let input:serde_json::Value=serde_json::from_str(include_str!("../../previus-rgps/tests/fixtures/synthetic-300-months.json")).unwrap();let mut input=input;input["opcoes"]["corrigirSalarios"]=json!(true);input["opcoes"]["divisorMinimo"]=json!(false);input["opcoes"]["descarte"]=json!(false);input["opcoes"]["referenciaCorrecao"]=json!("2024-12");input["dataReferencia"]=json!("2024-12-31");let outcome=previus_rgps::calculate_json(input).unwrap();assert_eq!(outcome["status"],"experimental");let rows=outcome["data"]["native_result"]["correcao"]["parcelas"].as_array().unwrap().len();let pdf=render_pdf(&case,&ws,opts(ReportKind::Memory,false),Some(&outcome)).unwrap();let(doc,text)=extract(&pdf);assert!(doc.get_pages().len()>4);assert!(rows>=300);assert!(text.contains("Todas as parcelas de correção"));assert!(text.contains("01/2000"));assert!(text.contains("12/2024"));}
#[test]fn unknown_answers_not_inferred_from_denied_benefit(){let(case,ws)=synthetic_case();let pdf=render_pdf(&case,&ws,opts(ReportKind::Detailed,false),None).unwrap();let(_,text)=extract(&pdf);assert!(text.contains("Não respondido"));assert!(text.contains("Não prova recebimento atual"));assert!(text.contains("Visitada / registrada"));}
#[test]fn oversized_values_error_instead_of_silent_truncation(){let(mut case,mut ws)=synthetic_case();case.name="x".repeat(1001);assert!(render_pdf(&case,&ws,opts(ReportKind::Summary,true),None).is_err());case.name="teste".into();ws.settings.report_footer="x".repeat(181);assert!(render_pdf(&case,&ws,opts(ReportKind::Summary,true),None).is_err());}
#[test]fn unsupported_unicode_is_error_not_a_missing_accent(){let(mut case,ws)=synthetic_case();case.name="Unsupported 😀".into();let e=match render_pdf(&case,&ws,opts(ReportKind::Summary,true),None){Err(error)=>error,Ok(_)=>panic!("Unsupported personal Unicode unexpectedly produced a PDF")};assert!(e.contains("Caractere não suportado"));assert!(!e.contains("😀"));assert!(render_pdf(&case,&ws,opts(ReportKind::Summary,false),None).is_ok());}
#[test]fn extreme_row_input_fails_closed(){let(mut case,ws)=synthetic_case();let line=case.cnis.as_ref().unwrap().salary_entries[0].clone();case.cnis.as_mut().unwrap().salary_entries=vec![line;10_001];assert!(render_pdf(&case,&ws,opts(ReportKind::Detailed,false),None).unwrap_err().contains("10.000"));}
#[test]fn deep_arbitrary_outcome_rejected(){let(case,ws)=synthetic_case();let mut value=json!({});for _ in 0..25{value=json!([value]);}assert!(render_pdf(&case,&ws,opts(ReportKind::Detailed,false),Some(&value)).is_err());}
#[test]fn invalid_source_month_not_published_or_guessed(){let(mut case,ws)=synthetic_case();case.cnis.as_mut().unwrap().salary_entries[0].competence="PERSONALMARKER".into();assert!(render_pdf(&case,&ws,opts(ReportKind::Detailed,false),None).is_err());}
#[test]fn forbidden_numeric_result_data_not_leaked(){let(case,ws,mut o)=calculated_case();o["data"]["native_result"]["resumo"]["rmi"]=json!("SECRET VALUE NAME");let err=render_pdf(&case,&ws,opts(ReportKind::Summary,false),Some(&o)).unwrap_err();assert!(!err.contains("SECRET"));}
#[test]fn conference_report_has_explicit_conclusion_and_considered_data(){
    let(case,ws)=synthetic_case();
    let pdf=render_pdf(&case,&ws,opts(ReportKind::Indicators,false),None).unwrap();
    let(_,text)=extract(&pdf);
    assert!(text.contains("RELATÓRIO DE CONFERÊNCIA"));
    assert!(text.contains("Dados efetivamente considerados"));
    assert!(text.contains("Nenhum resultado financeiro produzido"));
    assert!(text.contains("Qualidade de segurado - conferência por vínculo"));
    assert!(text.contains("não calcula período de graça"));
    assert!(text.to_lowercase().contains("desconsidera"));
}
#[test]fn experimental_report_leads_with_conclusion_and_source_status(){
    let(case,ws,o)=calculated_case();
    let pdf=render_pdf(&case,&ws,opts(ReportKind::Summary,false),Some(&o)).unwrap();
    let(_,text)=extract(&pdf);
    assert!(text.contains("Conclusão e o que pode mudá-la"));
    assert!(text.contains("Renda mensal inicial estimada"));
    assert!(text.contains("Dados efetivamente considerados"));
    assert!(text.contains("Experimental fornecido pelo motor"));
    assert!(!text.contains("RMI zero"));
}
#[test]fn planning_gap_is_declared_not_invented(){
    let(mut case,ws)=synthetic_case();
    case.assistant.as_mut().unwrap().process_kind=Some("Planejamento futuro".into());
    let pdf=render_pdf(&case,&ws,opts(ReportKind::Indicators,false),None).unwrap();
    let(_,text)=extract(&pdf);
    assert!(text.contains("Planejamento - premissas declaradas e lacunas"));
    assert!(text.contains("não projeta renda futura"));
    assert!(text.contains("Contribuição futura"));
    assert!(text.contains("Não implementada"));
    assert!(text.contains("Retorno do investimento"));
}
#[test]fn quality_review_keeps_grace_and_extensions_out_of_the_calculation(){
    let(case,ws)=synthetic_case();
    let pdf=render_pdf(&case,&ws,opts(ReportKind::Indicators,true),None).unwrap();
    let(_,text)=extract(&pdf);
    assert!(text.contains("Qualidade de segurado - conferência por vínculo"));
    assert!(text.contains("1º / último lançamento"));
    assert!(text.contains("Lanç."));
    assert!(text.contains("Desc."));
    assert!(text.contains("Filiação informada na fonte"));
    assert!(text.contains("Contribuições desconsideradas são as marcadas pela fonte"));
    assert!(!text.contains("período de graça calculado"));
}
#[test]fn operator_title_replaces_the_default_document_title(){
    let(mut case,ws)=synthetic_case();
    case.options.report_title="RMI — revisão de origem (2026)".into();
    let pdf=render_pdf(&case,&ws,opts(ReportKind::Detailed,false),None).unwrap();
    let(_,text)=extract(&pdf);
    assert!(text.contains("RMI — revisão de origem (2026)"));
    assert!(!text.contains("Relatório detalhado do cálculo previdenciário"));
}
#[test]fn an_oversized_operator_title_is_refused_instead_of_being_cut(){
    let(mut case,ws)=synthetic_case();
    case.options.report_title="x".repeat(121);
    assert!(render_pdf(&case,&ws,opts(ReportKind::Detailed,false),None).unwrap_err().contains("120"));
}
#[test]fn presentation_options_change_only_what_is_printed(){
    let(case,ws)=synthetic_case();
    let full=extract(&render_pdf(&case,&ws,opts(ReportKind::Detailed,false),None).unwrap()).1;
    assert!(full.contains("Anexo técnico - dados, origem e alcance"));
    assert!(full.contains("Qualidade de segurado - conferência por vínculo"));
    let mut options=opts(ReportKind::Detailed,false);
    options.presentation=previus_reports::Presentation{include_appendix:false,include_quality:false,include_planning:false};
    let trimmed=extract(&render_pdf(&case,&ws,options,None).unwrap()).1;
    assert!(!trimmed.contains("Anexo técnico - dados, origem e alcance"));
    assert!(!trimmed.contains("Qualidade de segurado - conferência por vínculo"));
    assert!(trimmed.contains("Síntese, situação e alcance"));
    assert!(trimmed.contains("Conclusão e o que pode mudá-la"));
}
