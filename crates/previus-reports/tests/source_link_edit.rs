mod common;
use common::{synthetic_case,extract};
use previus_workspace::{ManualLink};
use previus_workspace::link_review::{LinkSnapshot,LinkChange,LinkSourceReview};
use previus_reports::{render_pdf,ReportOptions,ReportKind};
#[test]fn native_pdf_keeps_original_and_adjusted_dates_and_hides_free_review_text(){
    let (mut c,w)=synthetic_case();let source=&c.cnis.as_ref().unwrap().links[0];
    let before=LinkSnapshot{employer:source.raw_employer.clone().unwrap_or_default(),start_date:source.start_date.clone().unwrap_or_default(),end_date:source.end_date.clone().unwrap_or_default(),regime:"RPPS".into(),indicators:source.indicators.join(", "),confirmed:false,excluded:false};
    let after=LinkSnapshot{end_date:"1990-12-31".into(),..before.clone()};
    c.manual_links.push(ManualLink{id:900,employer:after.employer.clone(),start_date:after.start_date.clone(),end_date:after.end_date.clone(),regime:after.regime.clone(),indicators:after.indicators.clone(),confirmed:false,source_review:Some(LinkSourceReview{sequence:source.sequence,source_sha256:c.cnis.as_ref().unwrap().metadata.source_sha256.clone(),excluded:false,history:vec![LinkChange{at:"2026-10-13T10:00:00Z".into(),reason:"SYNTHETIC_PRIVATE_LINK_REVIEW".into(),before,after}]})});
    let bytes=render_pdf(&c,&w,ReportOptions::new(ReportKind::Periods, false),None).unwrap();let(_,text)=extract(&bytes);assert!(text.contains("31/12/1990"));assert!(text.contains("Revisão de vínculos"));assert!(text.contains("Justificativa ocultada"));assert!(!text.contains("SYNTHETIC_PRIVATE_LINK_REVIEW"));assert!(c.cnis.as_ref().unwrap().links[0].end_date.is_none());
}
