//! Preparação assistida de uma hipótese RGPS a partir do CNIS importado.
//! As decisões são explícitas e registradas em notas; a fonte nunca é alterada.
use previdenciar_cnis::ImportResult;
use previdenciar_workspace::link_review::{LinkChange, LinkSnapshot, LinkSourceReview};
use previdenciar_workspace::{Calculation, ManualLink, ManualSalary};
use serde_json::json;
use std::collections::{BTreeSet, HashSet};

use crate::{competencia_pbc_anterior, parse_dib_date};

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
    let dib_date = parse_dib_date(&case.dib).ok_or("Informe uma DIB válida antes de preparar a hipótese.")?;
    let dib_iso = dib_date.format("%Y-%m-%d").to_string();
    let pbc_end = competencia_pbc_anterior(&case.dib)
        .ok_or("Não foi possível fixar o PBC anterior à DIB.")?;
    let prpps: HashSet<u32> = cnis.links.iter().filter(|link| link.prpps).map(|link| link.sequence).collect();
    let mut notes = Vec::new();
    let mut links = Vec::new();
    let mut sem_fim = Vec::new();
    let mut clamped = Vec::new();
    let mut excluded_after_dib = Vec::new();
    let mut fim_inferido = Vec::new();
    for link in cnis.links.iter().filter(|link| !link.prpps) {
        let Some(start_date) = link.start_date.clone() else {
            sem_fim.push(link.sequence);
            continue;
        };
        let source_end_date = link
            .end_date
            .clone()
            .or_else(|| link.inferred_last_salary_end.as_ref().map(|inferred| inferred.date_candidate.clone()))
            .or_else(|| ultima_competencia_do_vinculo(cnis, link.sequence).and_then(|competencia| fim_do_mes(&competencia)));
        let end_inferred = link.end_date.is_none();
        let Some(source_end_date) = source_end_date else {
            sem_fim.push(link.sequence);
            continue;
        };
        let excluded = start_date.as_str() > dib_iso.as_str();
        let end_date = if excluded {
            source_end_date.clone()
        } else if source_end_date.as_str() > dib_iso.as_str() {
            clamped.push(link.sequence);
            dib_iso.clone()
        } else {
            source_end_date.clone()
        };
        if excluded {
            excluded_after_dib.push(link.sequence);
        }
        if end_inferred && !excluded {
            fim_inferido.push(link.sequence);
        }
        let employer = link.raw_employer.clone().unwrap_or_default();
        let indicators = link.indicators.clone();
        let affiliation = link.affiliation.as_deref().unwrap_or("").trim();
        let affiliation_lower = affiliation.to_lowercase();
        let regime = if !affiliation.is_empty()
            && !affiliation_lower.contains("servidor")
            && !affiliation_lower.contains("rpps")
        {
            "RGPS"
        } else {
            "A confirmar"
        };
        let before = LinkSnapshot {
            employer: employer.clone(),
            start_date: link.start_date.clone().unwrap_or_default(),
            end_date: link.end_date.clone().unwrap_or_default(),
            regime: if link.prpps { "RPPS" } else { "A confirmar" }.into(),
            indicators: indicators.join(", "),
            confirmed: false,
            excluded: false,
        };
        // A referência documental informada é a conferência do CNIS: o vínculo
        // RGPS lido do documento entra confirmado. Regime indefinido continua pendente.
        let confirmed = regime == "RGPS" && !excluded;
        let after = LinkSnapshot {
            employer: employer.clone(),
            start_date: start_date.clone(),
            end_date: end_date.clone(),
            regime: regime.into(),
            indicators: indicators.join(", "),
            confirmed,
            excluded,
        };
        links.push(ManualLink {
            source_review: Some(LinkSourceReview {
                sequence: link.sequence,
                source_sha256: cnis.metadata.source_sha256.clone(),
                excluded,
                history: vec![LinkChange {
                    at: chrono::Utc::now().to_rfc3339(),
                    reason: format!("Preparação assistida a partir do CNIS conferido ({reference}): período efetivo limitado à DIB; indicadores preservados."),
                    before,
                    after,
                }],
            }),
            id: 0,
            employer,
            start_date,
            end_date,
            regime: regime.into(),
            indicators: indicators.join(", "),
            confirmed,
        });
    }
    // Benefício por incapacidade intercalado com contribuições conta como tempo
    // (Lei 8.213/1991, art. 55, II; Decreto 3.048/1999, art. 19-C, § 1º, XI).
    // Só entram benefícios cessados, com início e fim lidos do CNIS, que tenham
    // contribuição antes do início e depois do fim.
    let mut incapacidade = Vec::new();
    let mut nao_intercalados = 0;
    for benefit in &cnis.benefits {
        let especie_incapacidade = matches!(benefit.species, Some(31 | 32 | 91 | 92));
        if !especie_incapacidade || benefit.status != previdenciar_cnis::BenefitStatus::Ceased || benefit.included_as_employment_link {
            continue;
        }
        let (Some(start), Some(end)) = (benefit.start_date.clone(), benefit.end_date.clone()) else { continue };
        if end.as_str() > dib_iso.as_str() {
            continue;
        }
        let antes = cnis.salary_months.keys().any(|competencia| competencia.as_str() < &start[..7]);
        let depois = cnis.salary_months.keys().any(|competencia| competencia.as_str() > &end[..7]);
        if !(antes && depois) {
            nao_intercalados += 1;
            continue;
        }
        let especie = benefit.species.unwrap_or_default();
        incapacidade.push((benefit.sequence, especie, start.clone(), end.clone()));
        links.push(ManualLink {
            source_review: None,
            id: 0,
            employer: format!("Benefício por incapacidade (espécie {especie}) intercalado"),
            start_date: start,
            end_date: end,
            regime: "RGPS".into(),
            indicators: "BENEFICIO_INTERCALADO".into(),
            confirmed: true,
        });
    }
    if !incapacidade.is_empty() {
        notes.push(format!(
            "{} benefício(s) por incapacidade intercalado(s) contados como tempo de contribuição (Lei 8.213/1991, art. 55, II): {}.",
            incapacidade.len(),
            incapacidade.iter().map(|(seq, esp, ini, fim)| format!("seq. {seq}, espécie {esp}, {ini} a {fim}")).collect::<Vec<_>>().join("; ")
        ));
    }
    if nao_intercalados > 0 {
        notes.push(format!("{nao_intercalados} benefício(s) por incapacidade sem contribuição antes e depois ficaram fora do tempo."));
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
        .filter(|link| !link.source_review.as_ref().is_some_and(|review| review.excluded))
        .map(|link| (link.start_date[..7].to_string(), link.end_date[..7].to_string()))
        .collect();
    let mut fora_de_vinculo = 0;
    let mut pos_pbc = 0;
    let mut wages = Vec::new();
    let mut meses_sem_remuneracao = 0;
    let mut concomitantes = 0;
    for (competencia, mes) in &cnis.salary_months {
        if competencia.as_str() < "1994-07" {
            continue;
        }
        if competencia.as_str() > pbc_end.as_str() {
            pos_pbc += 1;
            continue;
        }
        let mut total: i64 = 0;
        let mut usados = 0;
        let mut indicators = BTreeSet::new();
        for indice in &mes.entry_indices {
            let Some(entry) = indice.checked_sub(1).and_then(|posicao| cnis.salary_entries.get(posicao)) else {
                continue;
            };
            if entry.link_sequence.is_some_and(|sequence| prpps.contains(&sequence)) {
                continue;
            }
            if !matches!(entry.amount_kind, previdenciar_cnis::AmountKind::Remuneration) {
                continue;
            }
            total = total
                .checked_add(entry.amount_centavos)
                .ok_or("Soma de remunerações excede o limite suportado.")?;
            usados += 1;
            for indicator in &entry.indicators {
                indicators.insert(indicator.clone());
            }
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
            indicators: indicators.into_iter().collect::<Vec<_>>().join(", "),
            // Valor nominal lido do CNIS conferido pela referência informada.
            confirmed: true,
        });
    }
    // Salários das competências cheias de auxílio por incapacidade intercalado
    // entram pelo salário mínimo vigente quando não há remuneração no CNIS
    // (Lei 8.213/1991, art. 29, § 5º; Decreto 3.048/1999, art. 36, § 7º).
    let mut existentes: HashSet<String> = wages.iter().map(|w| w.competence.clone()).collect();
    let mut competencias_intercaladas_sm = 0usize;
    for (_, _, start, end) in &incapacidade {
        if let (Ok(mut cur), Ok(fim_comp)) = (
            rgps_core::calendar::Competencia::parse(&start[..7], "beneficio.inicio"),
            rgps_core::calendar::Competencia::parse(&end[..7], "beneficio.fim"),
        ) {
            while cur <= fim_comp {
                let comp_str = cur.to_string();
                if comp_str.as_str() >= "1994-07"
                    && comp_str.as_str() <= pbc_end.as_str()
                    && !existentes.contains(&comp_str)
                {
                    if let Ok(sm) = rgps_core::atualizacao::salario_minimo_vigente(&cur) {
                        existentes.insert(comp_str.clone());
                        wages.push(ManualSalary {
                            id: 0,
                            competence: comp_str,
                            amount: rgps_core::money::money(sm),
                            indicators: "BENEFICIO_INTERCALADO_SM".into(),
                            confirmed: true,
                        });
                        competencias_intercaladas_sm += 1;
                    }
                }
                cur = cur.next();
            }
        }
    }
    wages.sort_by(|a, b| a.competence.cmp(&b.competence));
    if competencias_intercaladas_sm > 0 {
        notes.push(format!(
            "{competencias_intercaladas_sm} competência(s) de benefício por incapacidade intercalado sem remuneração no CNIS foram incluídas pelo salário mínimo vigente da época (Lei 8.213/1991, art. 29, § 5º; Decreto 3.048/1999, art. 36, § 7º). Confira o salário de benefício na carta de concessão."
        ));
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
    if !clamped.is_empty() {
        notes.push(format!(
            "{} vínculo(s) com fim posterior à DIB foram limitados a {} apenas no período efetivo de tempo; o fim original permanece na proveniência: {}.",
            clamped.len(),
            dib_iso,
            clamped.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
        ));
    }
    if !excluded_after_dib.is_empty() {
        notes.push(format!(
            "{} vínculo(s) iniciados após a DIB ficaram fora da hipótese, com origem preservada: {}.",
            excluded_after_dib.len(),
            excluded_after_dib.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
        ));
    }
    if pos_pbc > 0 {
        notes.push(format!(
            "{} competência(s) posteriores ao PBC ficaram fora da média; término oficial em {}, mês anterior à DIB (IN PRES/INSS 128/2022, art. 222, I e §1º).",
            pos_pbc,
            pbc_end
        ));
    }
    if !fim_inferido.is_empty() {
        notes.push(format!(
            "{} vínculo(s) sem data final no CNIS foram fechados pela última remuneração; confira: {}.",
            fim_inferido.len(),
            fim_inferido.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
        ));
    }
    let pendentes = links.iter().filter(|link| !link.confirmed && !link.source_review.as_ref().is_some_and(|r| r.excluded)).count();
    if pendentes > 0 {
        notes.push(format!("{pendentes} vínculo(s) com regime indefinido continuam pendentes de confirmação manual."));
    }
    notes.push(format!("Vínculos RGPS e remunerações do CNIS foram aceitos como conferidos pela referência \"{reference}\"; indicadores continuam visíveis para revisão."));
    let dib_mes = &dib_iso[..7];
    let ultimo_inpc = rgps_core::dados::inpc::INPC_MENSAL.last().map(|(periodo, _)| *periodo).unwrap_or("");
    if dib_mes > ultimo_inpc {
        notes.push(format!(
            "DIB ({dib_mes}) posterior ao último INPC publicado ({ultimo_inpc}): correção aplicada com índices publicados até {ultimo_inpc}, sem extrapolação futura."
        ));
    }
    let mut prepared = case.clone();
    if prepared.sex.as_deref().is_none_or(|v| v.trim().is_empty()) {
        if let Some(sex) = cnis.identity.sex.as_deref().filter(|s| !s.trim().is_empty()) {
            prepared.sex = Some(sex.to_string());
            notes.push("Sexo cadastral preenchido a partir do cabeçalho do CNIS.".into());
        }
    }
    prepared.manual_links = links;
    prepared.manual_salaries = wages;
    let mut native_options = json!({
        "corrigirSalarios": true,
        "descarte": false,
        "descarteOtimo": true,
        "divisorMinimo": false,
        "aplicarPisoAutomatico": true,
        "aplicarTetoVigenteDer": true,
        "referenciaCorrecao": dib_mes
    });
    if dib_iso.as_str() >= "1994-07-01" {
        native_options["perfilCorrecao"] = json!("trf3-v1");
    }
    prepared.options.rule_policy = serde_json::to_string(&json!({
        "version": crate::POLICY_VERSION,
        "review_evidence": reference,
        "manual_link_kind": "empregado",
        "manual_currency": "BRL",
        "exclude_imported_source": true,
        "source_exclusion_evidence": reference,
        "native_options": native_options
    }))
    .map_err(|_| "Não foi possível registrar a política da hipótese.".to_string())?;
    if prepared.disability.as_deref().map(str::trim).map(str::to_lowercase).is_none_or(|value| value.is_empty() || value == "a apurar") {
        notes.push("Condição de deficiência não declarada: confirme explicitamente antes de calcular; nenhum valor foi presumido.".into());
    }
    prepared.result = None;
    prepared.status = "Revisão pendente".into();
    if let Some(assistant) = prepared.assistant.as_mut() {
        if assistant.mode == "assisted" {
            // A preparação não responde à entrevista: pergunta sem resposta
            // continua pendente e a conferência mostra qual etapa corrigir.
            if assistant.current_benefit_received.is_none() {
                notes.push("A resposta sobre benefício atual continua pendente: a preparação não infere essa informação.".into());
            }
            assistant.review_confirmed = false;
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
