use crate::{blocked, calculate_json, issue, POLICY_VERSION};
use crate::contracts::*;
use previus_workspace::Calculation;
use serde_json::Value;

/// Bridge for the current private workspace. Only manually confirmed rows are
/// calculated. Import flags are NEVER promoted to validation by this bridge.
/// options.rule_policy carries typed JSON, not automatic parsing of free notes.
pub fn calculate_case(case: &Calculation) -> Result<Value,String> {
    let mut blockers = Vec::new();
    previus_workspace::link_review::validate_source_links(case)?;
    for link in &case.manual_links {
        if let Some(review)=&link.source_review {
            if !review.excluded && case.cnis.as_ref().is_some_and(|c|c.links.iter().any(|l|l.sequence==review.sequence && l.prpps)) {
                issue(&mut blockers,"source_ctc_proof_required","manual_links.source_review","A origem PRPPS exige CTC documental específica; editar o regime não comprova averbação RGPS.");
            }
        }
    }
    if let Some(review) = &case.contribution_review {
        review.validate(case)?;
        if !review.applied {
            issue(&mut blockers,"contribution_review_unapplied","contribution_review","O livro foi alterado; aplique a revisão aos salários antes de calcular.");
        }
    }
    if case.kind.trim().to_lowercase() != "rgps" {
        issue(&mut blockers,"case_kind_unsupported","kind","Este adaptador atende exclusivamente cálculos RGPS.");
    }
    let policy: CasePolicy = if case.options.rule_policy.trim().starts_with('{') {
        serde_json::from_str(&case.options.rule_policy).map_err(|e|format!("invalid_case_policy: {e}"))?
    } else {
        CasePolicy::default()
    };
    if policy.version.as_deref()!=Some(POLICY_VERSION) {
        issue(&mut blockers,"case_policy_missing","options.rule_policy","Informar JSON de política versionada e evidência de revisão; não inferir do nome do cenário.");
    }
    let evidence = policy.review_evidence.as_ref().is_some_and(|s|!s.trim().is_empty());
    if !evidence { issue(&mut blockers,"case_review_missing","options.rule_policy.review_evidence","Referência documental de revisão obrigatória."); }
    let excluded = policy.exclude_imported_source && policy.source_exclusion_evidence.as_ref().is_some_and(|s|!s.trim().is_empty());
    let mut warnings = Vec::new();
    if let Some(cnis)=&case.cnis {
        let identity_pairs=[(&case.identity.birth_date,&cnis.identity.birth_date),(&case.identity.cpf,&cnis.identity.cpf),(&case.identity.full_name,&cnis.identity.full_name)];
        if identity_pairs.iter().any(|(a,b)|a.is_some() && b.is_some() && a!=b) {
            issue(&mut blockers,"identity_conflict","identity","Identidade diverge da importação; confirmar sem sobrescrever.");
        }
        if !excluded {
            // There is no source-review structure in Calculation yet. Imported
            // ValidationStatus::Unvalidated stays unvalidated, even without flags.
            issue(&mut blockers,"source_review_required","cnis","CNIS importado ainda não possui revisão documental por registro. Usar períodos/salários manuais confirmados e exclusão explícita da fonte somente se documentalmente justificada.");
            if cnis.links.iter().any(|l|l.prpps || l.excluded_from_validated_domain) {
                issue(&mut blockers,"prpps_unvalidated","cnis.links","PRPPS/RPPS da fonte não será contado como RGPS.");
            }
            if cnis.links.iter().any(|l|l.end_date.is_none()) {
                issue(&mut blockers,"source_end_missing","cnis.links","Datas finais ausentes não são substituídas por último salário, DIB ou hoje.");
            }
            if !cnis.warnings.is_empty() || cnis.links.iter().any(|l|!l.indicators.is_empty()) || cnis.salary_entries.iter().any(|w|!w.indicators.is_empty()) {
                issue(&mut blockers,"source_indicators_unresolved","cnis","Avisos/indicadores da fonte continuam não resolvidos.");
            }
            if cnis.salary_entries.iter().any(|w|w.link_sequence.is_none()) {
                issue(&mut blockers,"source_wage_unassigned","cnis.salary_entries","Remuneração sem vínculo reconhecido.");
            }
        } else {
            warnings.push("Fonte CNIS excluída explicitamente deste cenário por decisão documental; não alterada. Somente linhas manuais confirmadas são usadas; isso não prova acerto da exclusão nem valida CTC.".into());
        }
    }
    if policy.exclude_imported_source && !excluded {
        issue(&mut blockers,"source_exclusion_not_documented","options.rule_policy.source_exclusion_evidence","Exclusão da fonte exige referência documental explícita.");
    }
    let opts=&case.options;
    if [(&opts.interest_rate,"interest_rate"),(&opts.interest_kind,"interest_kind"),(&opts.fees_percent,"fees_percent"),(&opts.fees_fixed,"fees_fixed"),(&opts.prescription_date,"prescription_date"),(&opts.reference_base,"reference_base")].iter().any(|(v,_)|!v.trim().is_empty()) || opts.include_thirteenth {
        issue(&mut blockers,"workspace_options_unsupported","options","Juros, honorários, prescrição, base manual e décimo terceiro não são usados pelo motor RGPS: não podem ser ignorados silenciosamente.");
    }
    if !opts.correction_index.trim().is_empty() && !opts.correction_index.trim().eq_ignore_ascii_case("INPC") {
        issue(&mut blockers,"correction_index_unsupported","options.correction_index","Motor copiado suporta INPC neste caminho, não outro índice arbitrário.");
    }
    let request = CalculationInput {
        segurado:InsuredInput {sexo:case.sex.clone(),data_nascimento:case.identity.birth_date.clone(),deficiencia:case.disability.clone()},
        data_referencia:Some(case.dib.clone()),
        vinculos:case.manual_links.iter().filter(|l|!l.source_review.as_ref().is_some_and(|r|r.excluded)).map(|l|LinkInput {
            inicio:Some(l.start_date.clone()),fim:Some(l.end_date.clone()),tipo:policy.manual_link_kind.clone(),
            regime:Some(l.regime.clone()),confirmado:l.confirmed,evidencia:policy.review_evidence.clone(),
            indicadores:if l.indicators.trim().is_empty(){vec![]}else{vec!["manual_indicator_unresolved".into()]},
            origem:Some("manual_documentado".into()),prpps:l.regime.trim().eq_ignore_ascii_case("PRPPS") || l.regime.trim().eq_ignore_ascii_case("RPPS"),
            // ManualLink has no CTC proof fields: it cannot confirm a CTC.
            ctc_confirmada:false,ctc_evidencia:None,fator:Some(1.0)
        }).collect(),
        competencias:case.manual_salaries.iter().map(|w|WageInput {
            competencia:Some(w.competence.clone()),salario:Some(w.amount.clone()),moeda:policy.manual_currency.clone(),confirmada:w.confirmed,
            indicadores:if w.indicators.trim().is_empty(){vec![]}else{vec!["manual_wage_indicator_unresolved".into()]}
        }).collect(),
        opcoes:policy.native_options,
        revisao:ReviewInput {confirmada:evidence, evidencia:policy.review_evidence, politica:policy.version},
    };
    // Unsafe source never reaches native calculation, not even temporarily.
    if !blockers.is_empty() { return Ok(blocked(blockers,warnings)); }
    let mut outcome=calculate_json(serde_json::to_value(request).map_err(|e|e.to_string())?)?;
    if !warnings.is_empty() {
        outcome["warnings"].as_array_mut().expect("adapter envelope").extend(warnings.into_iter().map(Value::String));
    }
    outcome["bridge"]=serde_json::json!({"input":"previus_workspace::Calculation","source_import_mutated":false,"manual_only":true,"category_from_explicit_case_policy":true});
    outcome["warnings"].as_array_mut().expect("adapter envelope").push(Value::String("Categoria e moeda provêm de declaração explícita em options.rule_policy; ManualLink não armazena CTC documental.".into()));
    Ok(outcome)
}
