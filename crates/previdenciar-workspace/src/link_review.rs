//! Documentary link overlays leave the imported source unchanged.
use crate::{Calculation,ManualLink};
use serde::{Serialize,Deserialize};
use std::collections::{HashSet,HashMap};
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkSnapshot{pub employer:String,pub start_date:String,pub end_date:String,pub regime:String,pub indicators:String,pub confirmed:bool,pub excluded:bool}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkChange{pub at:String,pub reason:String,pub before:LinkSnapshot,pub after:LinkSnapshot}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkSourceReview{pub sequence:u32,pub source_sha256:Option<String>,pub excluded:bool,pub history:Vec<LinkChange>}
fn snapshot(link:&ManualLink)->LinkSnapshot{LinkSnapshot{employer:link.employer.clone(),start_date:link.start_date.clone(),end_date:link.end_date.clone(),regime:link.regime.clone(),indicators:link.indicators.clone(),confirmed:link.confirmed,excluded:link.source_review.as_ref().is_some_and(|r|r.excluded)}}
fn validate_snapshot(s:&LinkSnapshot)->Result<(),String>{
    if s.employer.chars().count()>1000||s.indicators.chars().count()>1000||!matches!(s.regime.as_str(),"A confirmar"|"RGPS"|"RPPS"|"CTC a revisar"){return Err("Campos do ajuste de vínculo inválidos.".into());}
    for date in [&s.start_date,&s.end_date]{if !date.is_empty()&&(date.len()!=10||chrono::NaiveDate::parse_from_str(date,"%Y-%m-%d").is_err()){return Err("Data declarada do vínculo inválida.".into());}}
    if !s.start_date.is_empty()&&!s.end_date.is_empty()&&s.start_date>s.end_date{return Err("Fim do vínculo anterior ao início.".into());}Ok(())
}
pub fn validate_source_links(case:&Calculation)->Result<(),String>{
    let mut sequences=HashSet::new();
    let sources:HashMap<_,_>=case.cnis.as_ref().map(|c|c.links.iter().map(|l|(l.sequence,l)).collect()).unwrap_or_default();
    let mut operations=0usize;
    for link in &case.manual_links{
        let Some(review)=&link.source_review else{continue};
        if !sequences.insert(review.sequence)||review.history.len()>200{return Err("Ajuste de vínculo duplicado ou histórico excedido.".into());}
        let cnis=case.cnis.as_ref().ok_or("Ajuste de vínculo sem CNIS original.")?;
        if review.source_sha256!=cnis.metadata.source_sha256{return Err("Ajuste de vínculo aponta a outro documento.".into());}
        let source=sources.get(&review.sequence).ok_or("Vínculo de origem não encontrado.")?;
        operations=operations.checked_add(review.history.len()).ok_or("Histórico de vínculos excede limite.")?;if operations>50000{return Err("Histórico de vínculos excede limite.".into());}
        let mut indicators=source.indicators.clone();if source.prpps&&!indicators.iter().any(|v|v=="PRPPS"){indicators.push("PRPPS".into());}
        let initial=LinkSnapshot{employer:source.raw_employer.clone().unwrap_or_default(),start_date:source.start_date.clone().unwrap_or_default(),end_date:source.end_date.clone().unwrap_or_default(),regime:if source.prpps{"RPPS"}else{"A confirmar"}.into(),indicators:indicators.join(", "),confirmed:false,excluded:false};
        // Old imported dates can be incomplete; only edited snapshots must be valid.
        let mut state=initial;
        for change in &review.history{
            if change.reason.trim().is_empty()||change.reason.chars().count()>1000||change.at.len()>64||chrono::DateTime::parse_from_rfc3339(&change.at).is_err(){return Err("Ajuste de vínculo exige motivo e data de revisão válidos.".into());}
            if state!=change.before{return Err("Histórico de vínculo não corresponde ao estado anterior.".into());}
            validate_snapshot(&change.after)?;state=change.after.clone();
        }
        if state!=snapshot(link){return Err("Vínculo alterado sem operação documentada.".into());}
        if (link.confirmed||review.excluded)&&review.history.is_empty(){return Err("Conferência ou exclusão de vínculo exige justificativa.".into());}
    }Ok(())
}
