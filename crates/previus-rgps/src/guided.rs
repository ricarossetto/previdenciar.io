//! Preparação assistida de uma hipótese RGPS a partir do CNIS importado.
//! As decisões são explícitas e registradas em notas; a fonte nunca é alterada.
use previus_cnis::ImportResult;
use previus_workspace::link_review::{LinkChange, LinkSnapshot, LinkSourceReview};
use previus_workspace::{Calculation, ManualLink, ManualSalary};
use serde_json::json;
use std::collections::HashSet;

fn fim_do_mes(competencia: &str) -> Option<String> {
    let ano: i32 = competencia.get(0..4)?.parse().ok()?;
    let mes: u32 = competencia.get(5..7)?.parse().ok()?;
    let (proximo_ano, proximo_mes) = if mes == 12 { (ano + 1, 1) } else { (ano, mes + 1) };
    let primeiro = chrono::NaiveDate::from_ymd_opt(proximo_ano, proximo_mes, 1)?;
    Some(primeiro.pred_opt()?.format("%Y-%m-%d").to_string())
}

fn ultima_competencia_do_vinculo(cnis: &ImportResult, sequence: u32) -> Option<String> {
    cnis.salary_entries
        .iter()
        .filter(|entry| entry.link_sequence == Some(sequence))
        .map(|entry| entry.competence.clone())
        .max()
}

/// Aplica as decisões de revisão comuns a um CNIS real: exclui vínculos PRPPS,
/// fecha vínculos sem data final pelo último salário, soma remunerações
/// concomitantes do RGPS e registra tudo em notas. Não altera o CNIS.
pub fn prepare_from_cnis(case: &Calculation, reference: &str) -> Result<(Calculation, Vec<String>), String> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err("Informe a referência documental da conferência antes de preparar a hipótese.".into());
    }
    let cnis = case.cnis.as_ref().ok_or("Importe um CNIS antes de preparar a hipótese.")?;
    if case.dib.len() < 7 {
        return Err("Informe uma DIB válida antes de preparar a hipótese.".into());
    }
    let prpps: HashSet<u32> = cnis.links.iter().filter(|link| link.prpps).map(|link| link.sequence).collect();
    let mut notes = Vec::new();
    let mut links = Vec::new();
    let mut sem_fim = Vec::new();
    for link in cnis.links.iter().filter(|link| !link.prpps) {
        let Some(start_date) = link.start_date.clone() else {
            sem_fim.push(link.sequence);
            continue;
        };
        let end_date = link
            .end_date
            .clone()
            .or_else(|| link.inferred_last_salary_end.as_ref().map(|inferred| inferred.date_candidate.clone()))
            .or_else(|| ultima_competencia_do_vinculo(cnis, link.sequence).and_then(|competencia| fim_do_mes(&competencia)));
        let Some(end_date) = end_date else {
            sem_fim.push(link.sequence);
            continue;
        };
        let employer = link.raw_employer.clone().unwrap_or_default();
        let mut indicators = link.indicators.clone();
        if link.prpps && !indicators.iter().any(|value| value == "PRPPS") {
            indicators.push("PRPPS".into());
        }
        let before = LinkSnapshot {
            employer: employer.clone(),
            start_date: link.start_date.clone().unwrap_or_default(),
            end_date: link.end_date.clone().unwrap_or_default(),
            regime: if link.prpps { "RPPS" } else { "A confirmar" }.into(),
            indicators: indicators.join(", "),
            confirmed: false,
            excluded: false,
        };
        let after = LinkSnapshot {
            employer: employer.clone(),
            start_date: start_date.clone(),
            end_date: end_date.clone(),
            regime: "RGPS".into(),
            indicators: String::new(),
            confirmed: true,
            excluded: false,
        };
        links.push(ManualLink {
            source_review: Some(LinkSourceReview {
                sequence: link.sequence,
                source_sha256: cnis.metadata.source_sha256.clone(),
                excluded: false,
                history: vec![LinkChange {
                    at: chrono::Utc::now().to_rfc3339(),
                    reason: "Preparação assistida a partir do CNIS: fim pelo último salário, indicadores informativos limpos e confirmação documental.".into(),
                    before,
                    after,
                }],
            }),
            id: 0,
            employer,
            start_date,
            end_date,
            regime: "RGPS".into(),
            indicators: String::new(),
            confirmed: true,
        });
    }
    if !prpps.is_empty() {
        notes.push(format!(
            "{} vínculo(s) PRPPS excluídos: regime próprio exige CTC e este motor calcula somente RGPS.",
            prpps.len()
        ));
    }
    if !sem_fim.is_empty() {
        notes.push(format!(
            "{} vínculo(s) sem data final e sem último salário ficaram fora: {}.",
            sem_fim.len(),
            sem_fim.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
        ));
    }
    let cobertura: Vec<(String, String)> = links
        .iter()
        .map(|link| (link.start_date[..7].to_string(), link.end_date[..7].to_string()))
        .collect();
    let mut fora_de_vinculo = 0;
    let mut wages = Vec::new();
    let mut meses_sem_remuneracao = 0;
    let mut concomitantes = 0;
    for (competencia, mes) in &cnis.salary_months {
        if competencia.as_str() < "1994-07" {
            continue;
        }
        let mut total: i64 = 0;
        let mut usados = 0;
        for indice in &mes.entry_indices {
            let Some(entry) = indice.checked_sub(1).and_then(|posicao| cnis.salary_entries.get(posicao)) else {
                continue;
            };
            if entry.link_sequence.is_some_and(|sequence| prpps.contains(&sequence)) {
                continue;
            }
            if !matches!(entry.amount_kind, previus_cnis::AmountKind::Remuneration) {
                continue;
            }
            total = total
                .checked_add(entry.amount_centavos)
                .ok_or("Soma de remunerações excede o limite suportado.")?;
            usados += 1;
        }
        if usados == 0 {
            meses_sem_remuneracao += 1;
            continue;
        }
        let coberto = cobertura
            .iter()
            .any(|(inicio, fim)| competencia.as_str() >= inicio.as_str() && competencia.as_str() <= fim.as_str());
        if !coberto {
            fora_de_vinculo += 1;
            continue;
        }
        if usados > 1 {
            concomitantes += 1;
        }
        wages.push(ManualSalary {
            id: 0,
            competence: competencia.clone(),
            amount: format!("{}.{:02}", total / 100, total % 100),
            indicators: String::new(),
            confirmed: true,
        });
    }
    if meses_sem_remuneracao > 0 {
        notes.push(format!(
            "{} mês(es) sem remuneração RGPS (PRPPS ou coluna de contribuição) ficaram fora do PBC.",
            meses_sem_remuneracao
        ));
    }
    if fora_de_vinculo > 0 {
        notes.push(format!(
            "{} competência(s) sem vínculo RGPS confirmado (ou cobertas apenas por PRPPS) ficaram fora do PBC.",
            fora_de_vinculo
        ));
    }
    if concomitantes > 0 {
        notes.push(format!(
            "{} mês(es) com mais de uma remuneração RGPS foram somados; confira a concomitância na origem.",
            concomitantes
        ));
    }
    let dib_mes = &case.dib[..7];
    let ultimo_inpc = rgps_core::dados::inpc::INPC_MENSAL.last().map(|(periodo, _)| *periodo).unwrap_or("");
    let corrigir = dib_mes <= ultimo_inpc;
    if !corrigir {
        notes.push("Sem INPC publicado para o mês da DIB: correção desativada; o resultado é nominal e não serve como memória pronta para protocolo.".into());
    }
    let mut prepared = case.clone();
    prepared.manual_links = links;
    prepared.manual_salaries = wages;
    prepared.options.rule_policy = serde_json::to_string(&json!({
        "version": crate::POLICY_VERSION,
        "review_evidence": reference,
        "manual_link_kind": "empregado",
        "manual_currency": "BRL",
        "exclude_imported_source": true,
        "source_exclusion_evidence": reference,
        "native_options": {
            "corrigirSalarios": corrigir,
            "descarte": false,
            "divisorMinimo": false,
            "aplicarTetoVigenteDer": true,
            "referenciaCorrecao": if corrigir { serde_json::Value::String(dib_mes.to_string()) } else { serde_json::Value::Null }
        }
    }))
    .map_err(|_| "Não foi possível registrar a política da hipótese.".to_string())?;
    if prepared.disability.as_deref().map(str::trim).map(str::to_lowercase).is_none_or(|value| value.is_empty() || value == "a apurar") {
        prepared.disability = Some("none".into());
        notes.push("Condição de deficiência não declarada: a hipótese foi preparada como sem deficiência. Altere se houver laudo ou grau reconhecido.".into());
    }
    prepared.result = None;
    prepared.status = "Revisão pendente".into();
    if let Some(assistant) = prepared.assistant.as_mut() {
        if assistant.mode == "assisted" {
            assistant.current_benefit_received.get_or_insert(false);
            assistant.process_kind.get_or_insert_with(|| "Simulação do benefício".into());
            assistant.goal.get_or_insert_with(|| "Calcular um novo benefício".into());
            assistant.review_confirmed = true;
            for step in ["intro", "benefit", "import", "process", "goal", "parameters", "periods", "salaries", "policy", "review"] {
                if !assistant.completed_steps.iter().any(|item| item == step) {
                    assistant.completed_steps.push(step.into());
                }
            }
        }
    }
    notes.push(format!(
        "Hipótese preparada com {} vínculo(s) e {} competência(s); o CNIS permanece intacto. Referência: {}.",
        prepared.manual_links.len(),
        prepared.manual_salaries.len(),
        reference
    ));
    Ok((prepared, notes))
}
