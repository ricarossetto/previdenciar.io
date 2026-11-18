use crate::{blocked, calculate_json, issue, POLICY_VERSION};
use crate::contracts::*;
use previdenciar_workspace::Calculation;
use serde_json::Value;
use std::collections::HashMap;

fn tipo_do_vinculo(afiliacao: Option<&str>) -> String {
    let texto = afiliacao.unwrap_or("").to_lowercase();
    if texto.contains("doméstico") || texto.contains("domestico") {
        "empregado_domestico"
    } else if texto.contains("contribuinte individual") {
        "contribuinte_individual"
    } else if texto.contains("avulso") {
        "avulso"
    } else if texto.contains("facultativo") {
        "facultativo"
    } else if texto.contains("rural") {
        "rural"
    } else if texto.contains("servidor") {
        "servidor"
    } else if texto.contains("empregado") || texto.contains("agente") {
        "empregado"
    } else {
        ""
    }
    .to_string()
}

/// Bridge for the current private workspace. Only manually confirmed rows are
/// calculated. Import flags are NEVER promoted to validation by this bridge.
/// options.rule_policy carries typed JSON, not automatic parsing of free notes.
/// Calcula o caso e, quando nenhuma regra está elegível na DIB, informa
/// "sem direito na DIB" e a projeção de cada regra.
pub fn calculate_case(case: &Calculation) -> Result<Value,String> {
    let mut outcome = calcular_na_dib(case)?;
    // Descarte pode tirar o tempo mínimo de uma regra: sem regra elegível,
    // refaz sem descarte antes de concluir que não há direito.
    let sem_regra = |o: &Value| o["data"]["native_result"]["transicoes"]["regras"].is_array() && o["data"]["native_result"]["transicoes"]["regra_escolhida"].is_null();
    if sem_regra(&outcome) {
        if let Ok(mut policy) = serde_json::from_str::<Value>(&case.options.rule_policy) {
            if policy["native_options"]["descarteOtimo"] == true {
                policy["native_options"]["descarteOtimo"] = Value::Bool(false);
                let mut sem = case.clone();
                sem.options.rule_policy = policy.to_string();
                if let Ok(alt) = calcular_na_dib(&sem) {
                    if !sem_regra(&alt) && alt["status"] != "blocked" {
                        outcome = alt;
                        outcome["warnings"].as_array_mut().map(|w| w.push(Value::String("Descarte das menores contribuições não aplicado: ele reduziria o tempo abaixo do mínimo da regra.".into())));
                    }
                }
            }
        }
    }
    let transicoes = &outcome["data"]["native_result"]["transicoes"];
    if transicoes["regras"].is_array() {
        let direito = !transicoes["regra_escolhida"].is_null();
        outcome["data"]["direito_na_dib"] = Value::Bool(direito);
        if !direito {
            if let Some(projecao) = project_rules(case, &outcome) {
                outcome["data"]["projecao_regras"] = projecao;
            }
        }
    }
    Ok(outcome)
}

fn calcular_na_dib(case: &Calculation) -> Result<Value,String> {
    if case.options.rule_policy.trim().starts_with('{') {
        let policy:CasePolicy=serde_json::from_str(&case.options.rule_policy).map_err(|_|"Política de cálculo inválida.")?;
        if let Some(pcd)=policy.lc142 { return crate::pcd::calculate(case,&pcd); }
    }
    let is_pcd = case.disability.as_deref().is_some_and(|value|matches!(value.trim().to_lowercase().as_str(),"grave"|"moderada"|"leve"));
    if is_pcd {
        let policy: CasePolicy = if case.options.rule_policy.trim().starts_with('{') {
            serde_json::from_str(&case.options.rule_policy).map_err(|_| "Política de cálculo inválida.")?
        } else {
            CasePolicy::default()
        };
        let evidence = policy.review_evidence.clone().unwrap_or_default();
        let has_review = policy.version.as_deref() == Some(POLICY_VERSION) && !evidence.trim().is_empty();
        if has_review && crate::parse_dib_date(&case.dib).is_some() {
            let dib_iso = crate::parse_dib_date(&case.dib).unwrap().format("%Y-%m-%d").to_string();
            let dib_month = dib_iso[..7].to_string();
            let ultimo_inpc = rgps_core::dados::inpc::INPC_MENSAL.last().map(|(p, _)| *p).unwrap_or("2026-09");
            let snap_through = if dib_month.as_str() <= ultimo_inpc { dib_month.as_str() } else { ultimo_inpc };
            let snapshot = crate::pcd::local_snapshot(snap_through).ok();
            let auto_pcd = crate::pcd::Lc142Policy {
                version: crate::pcd::VERSION.into(),
                calculation_date: dib_iso,
                correction_month: dib_month,
                assumptions_confirmed: true,
                review_evidence: evidence.clone(),
                disability_all_periods: true,
                disability_evidence: evidence,
                rgps_confirmed: true,
                project_open_links: false,
                ongoing_evidence: String::new(),
                fill_missing_minimum: false,
                missing_salary_evidence: String::new(),
                project_at_known_ceiling: false,
                projection_evidence: String::new(),
                historical_negative_policy: "preserve".into(),
                apply_inss_overrides: false,
                disability_start_date: None,
                fator_conversao_comum: None,
                allow_confirmed_manual_without_book: !case.manual_salaries.is_empty()
                    && case.manual_salaries.iter().all(|w| w.confirmed),
                snapshot,
            };
            return crate::pcd::calculate(case, &auto_pcd);
        }
        let mut pcd_blockers = Vec::new();
        if !has_review {
            issue(&mut pcd_blockers, "lc142_review", "options.rule_policy", "Confirme as premissas e a referência documental para cálculo PcD (LC 142/2013).");
        }
        if crate::parse_dib_date(&case.dib).is_none() {
            issue(&mut pcd_blockers, "lc142_dib", "dib", "Informe uma DIB válida (AAAA-MM-DD) para cálculo PcD (LC 142/2013).");
        }
        if case.identity.birth_date.as_deref().is_none_or(|s| s.trim().is_empty()) {
            issue(&mut pcd_blockers, "lc142_birth", "identity.birth_date", "Confirme a data de nascimento para cálculo PcD (LC 142/2013).");
        }
        return Ok(blocked(pcd_blockers, vec![]));
    }
    let mut blockers = Vec::new();
    previdenciar_workspace::link_review::validate_source_links(case)?;
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
    if let Some(especie) = case.especie.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        match especie {
            "42" | "41" | "31" | "32" | "46" | "57" => {}
            outra => {
                let mensagem = format!(
                    "Espécie {outra} não tem fórmula implementada. Espécies atendidas: 42 (tempo de contribuição), 41 (idade), 32 (incapacidade permanente), 31 (incapacidade temporária), 46 (especial) e 57 (professor)."
                );
                issue(&mut blockers, "especie_unsupported", "especie", &mensagem);
            }
        }
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
        if identity_pairs.iter().any(|(a,b)|a.is_some() && b.is_some() && a!=b) && !policy.identity_confirmed {
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
    let tipos: HashMap<u32, String> = case
        .cnis
        .as_ref()
        .map(|cnis| {
            cnis.links
                .iter()
                .map(|link| (link.sequence, tipo_do_vinculo(link.affiliation.as_deref())))
                .collect()
        })
        .unwrap_or_default();
    let request = CalculationInput {
        segurado:InsuredInput {sexo:case.sex.clone(),data_nascimento:case.identity.birth_date.clone(),deficiencia:case.disability.clone(),especie:case.especie.clone()},
        data_referencia:Some(case.dib.clone()),
        vinculos:case.manual_links.iter().filter(|l|!l.source_review.as_ref().is_some_and(|r|r.excluded)).map(|l|LinkInput {
            inicio:Some(l.start_date.clone()),fim:Some(l.end_date.clone()),tipo:l.source_review.as_ref().and_then(|review| tipos.get(&review.sequence).cloned()).filter(|value| !value.trim().is_empty()).or_else(|| policy.manual_link_kind.clone()),
            regime:Some(l.regime.clone()),confirmado:l.confirmed,evidencia:policy.review_evidence.clone(),
            indicadores:l.indicators.split(',').map(str::trim).filter(|s|!s.is_empty()).map(str::to_string).collect(),
            origem:Some("manual_documentado".into()),prpps:l.regime.trim().eq_ignore_ascii_case("PRPPS") || l.regime.trim().eq_ignore_ascii_case("RPPS"),
            // ManualLink has no CTC proof fields: it cannot confirm a CTC.
            ctc_confirmada:false,ctc_evidencia:None,fator:Some(1.0)
        }).collect(),
        competencias:case.manual_salaries.iter().map(|w|WageInput {
            competencia:Some(w.competence.clone()),salario:Some(w.amount.clone()),moeda:policy.manual_currency.clone(),confirmada:w.confirmed,
            indicadores:w.indicators.split(',').map(str::trim).filter(|s|!s.is_empty()).map(str::to_string).collect()
        }).collect(),
        opcoes:policy.native_options,
        contribution_months:case.cnis.as_ref().map(|cnis|cnis.contribution_months.clone()).unwrap_or_default(),
        revisao:ReviewInput {confirmada:evidence, evidencia:policy.review_evidence, politica:policy.version},
    };
    // Unsafe source never reaches native calculation, not even temporarily.
    if !blockers.is_empty() { return Ok(blocked(blockers,warnings)); }
    let mut outcome=calculate_json(serde_json::to_value(request).map_err(|e|e.to_string())?)?;
    if !warnings.is_empty() {
        outcome["warnings"].as_array_mut().expect("adapter envelope").extend(warnings.into_iter().map(Value::String));
    }
    outcome["bridge"]=serde_json::json!({"input":"previdenciar_workspace::Calculation","source_import_mutated":false,"manual_only":true,"category_from_explicit_case_policy":true});
    outcome["warnings"].as_array_mut().expect("adapter envelope").push(Value::String("Categoria e moeda provêm de declaração explícita em options.rule_policy; ManualLink não armazena CTC documental.".into()));
    Ok(outcome)
}

/// Project future DER from current confirmed data. Returns the first eligible
/// rule and RMI. This is a planning tool, not a calculation or entitlement.
pub fn project_case(case: &Calculation) -> Result<Value, String> {
    let policy: CasePolicy = if case.options.rule_policy.trim().starts_with('{') {
        serde_json::from_str(&case.options.rule_policy).map_err(|e| format!("invalid_case_policy: {e}"))?
    } else {
        CasePolicy::default()
    };
    let sex = case.sex.as_deref().unwrap_or("").trim().to_lowercase();
    if sex != "feminino" && sex != "masculino" {
        return Err("Sexo cadastrual obrigatório para projeção.".into());
    }
    let birth = case.identity.birth_date.as_deref().ok_or("Data de nascimento obrigatória para projeção.")?;
    let birth_date = rgps_core::calendar::IsoDate::parse(birth, "project.birth").map_err(|e| e.to_string())?;
    let referencia = rgps_core::calendar::IsoDate::parse(&case.dib, "project.dib").map_err(|e| e.to_string())?;
    // Build competencias from confirmed manual salaries or CNIS.
    let mut competencias: Vec<(rgps_core::calendar::Competencia, rust_decimal::Decimal)> = Vec::new();
    if !case.manual_salaries.is_empty() {
        for w in &case.manual_salaries {
            if !w.confirmed { continue; }
            let comp = rgps_core::calendar::Competencia::parse(&w.competence, "project.wage").map_err(|e| e.to_string())?;
            let amount = rgps_core::money::parse_money(&w.amount, "project.wage").map_err(|e| e.to_string())?;
            competencias.push((comp, amount));
        }
    } else if let Some(cnis) = &case.cnis {
        for entry in &cnis.salary_entries {
            let comp = rgps_core::calendar::Competencia::parse(&entry.competence, "project.cnis").map_err(|e| e.to_string())?;
            let amount = rust_decimal::Decimal::from(entry.amount_centavos) / rust_decimal::Decimal::from(100);
            competencias.push((comp, amount));
        }
    }
    if competencias.is_empty() {
        return Err("Projeção exige ao menos uma competência confirmada.".into());
    }
    competencias.sort_by_key(|(c, _)| (c.ano, c.mes));
    let opts = &policy.native_options;
    let parse_dec = |v: &Option<String>| -> Result<Option<rust_decimal::Decimal>, String> {
        match v {
            Some(s) if !s.trim().is_empty() => rgps_core::money::parse_money(s, "project.opt").map(Some).map_err(|e| e.0),
            _ => Ok(None),
        }
    };
    let req = rgps_core::projecao::ProjecaoRequisicao {
        sexo: sex,
        data_nascimento: birth_date,
        data_referencia: referencia,
        competencias,
        corrigir_salarios: opts.corrigir_salarios.unwrap_or(false),
        descarte: opts.descarte.unwrap_or(false),
        salario_simulacao: None,
        teto: parse_dec(&opts.teto)?,
        piso: parse_dec(&opts.piso)?,
        aplicar_teto_vigente_der: opts.aplicar_teto_vigente_der.unwrap_or(false),
        expectativa_sobrevida: parse_dec(&opts.expectativa_sobrevida)?,
        regra_alvo: opts.regra_alvo.clone(),
    };
    let resultado = rgps_core::projecao::projetar(&req).map_err(|e| e.0)?;
    serde_json::to_value(&resultado).map_err(|e| e.to_string())
}

fn somar_meses(data: chrono::NaiveDate, meses: u32) -> Option<chrono::NaiveDate> {
    data.checked_add_months(chrono::Months::new(meses))
}

/// Projeta, mês a mês após a DIB, a primeira data em que cada regra de
/// transição fica elegível. Premissa declarada: contribuição contínua com o
/// último salário conferido, sem reajuste real. É planejamento, não direito.
pub fn project_rules(case: &Calculation, outcome: &Value) -> Option<Value> {
    const LIMITE_MESES: u32 = 180;
    let regras = outcome["data"]["native_result"]["transicoes"]["regras"].as_array()?;
    let mut pendentes: Vec<String> = regras
        .iter()
        .filter(|r| r["elegivel"] != true && r["id"] != "direito-adquirido" && r["faltas"]["aplicavel"] != false)
        .filter_map(|r| r["id"].as_str().map(str::to_string))
        .collect();
    if pendentes.is_empty() {
        return None;
    }
    let dib = crate::parse_dib_date(&case.dib)?;
    let ultimo = case.manual_salaries.iter().filter(|w| w.confirmed).max_by(|a, b| a.competence.cmp(&b.competence))?;
    let fim_vinculos = case
        .manual_links
        .iter()
        .filter(|l| l.confirmed && !l.source_review.as_ref().is_some_and(|r| r.excluded))
        .filter_map(|l| chrono::NaiveDate::parse_from_str(&l.end_date, "%Y-%m-%d").ok())
        .max()?;
    let mut policy: Value = serde_json::from_str(&case.options.rule_policy).ok()?;
    let simular_com = |data: chrono::NaiveDate, descarte: bool| -> Option<(String, Value)> {
        let mut policy = policy.clone();
        policy["native_options"]["descarteOtimo"] = Value::Bool(descarte);
        let mut simulado = case.clone();
        simulado.dib = data.format("%Y-%m-%d").to_string();
        let competencia_dib = data.format("%Y-%m").to_string();
        policy["native_options"]["referenciaCorrecao"] = Value::String(competencia_dib.clone());
        simulado.options.rule_policy = policy.to_string();
        if data > fim_vinculos {
            simulado.manual_links.push(previdenciar_workspace::ManualLink {
                id: 0,
                employer: "Projeção: contribuição contínua".into(),
                start_date: fim_vinculos.succ_opt()?.format("%Y-%m-%d").to_string(),
                end_date: simulado.dib.clone(),
                regime: "RGPS".into(),
                indicators: "PROJECAO".into(),
                confirmed: true,
                source_review: None,
            });
        }
        // Salários projetados até o mês anterior à DIB simulada.
        let mut competencia = chrono::NaiveDate::parse_from_str(&format!("{}-01", ultimo.competence), "%Y-%m-%d").ok()?;
        loop {
            competencia = somar_meses(competencia, 1)?;
            let texto = competencia.format("%Y-%m").to_string();
            if texto >= competencia_dib {
                break;
            }
            simulado.manual_salaries.push(previdenciar_workspace::ManualSalary {
                id: 0,
                competence: texto,
                amount: ultimo.amount.clone(),
                indicators: "PROJECAO".into(),
                confirmed: true,
            });
        }
        calcular_na_dib(&simulado).ok().map(|r| (simulado.dib, r))
    };
    // O descarte (EC 103, art. 26, § 6º) reduz o tempo e pode tirar o requisito de
    // outra regra; por isso cada regra é avaliada com e sem descarte, e vale o
    // cenário elegível de maior RMI.
    let simular = |data: chrono::NaiveDate| -> Option<(String, Value)> {
        let com = simular_com(data, true);
        let sem = simular_com(data, false);
        match (com, sem) {
            (Some((d, mut a)), Some((_, b))) => {
                let tempo_com = a["data"]["native_result"]["transicoes"]["tempo_total_meses"].as_i64().unwrap_or(0);
                let tempo_base = if case.sex.as_deref().is_some_and(|s| s.trim().eq_ignore_ascii_case("feminino")) { 180 } else { 240 };
                if let (Some(ra), Some(rb)) = (
                    a["data"]["native_result"]["transicoes"]["regras"].as_array_mut(),
                    b["data"]["native_result"]["transicoes"]["regras"].as_array(),
                ) {
                    for regra in ra.iter_mut() {
                        let Some(outra) = rb.iter().find(|x| x["id"] == regra["id"]) else { continue };
                        let valor = |r: &Value| r["rmi_final"].as_str().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                        // Como no Prévius: o descarte não pode deixar o tempo abaixo do
                        // tempo-base do coeficiente (20 anos homem, 15 anos mulher).
                        let descarte_valido = tempo_com >= tempo_base;
                        let usar_outra = outra["elegivel"] == true && (!descarte_valido || regra["elegivel"] != true || valor(outra) > valor(regra));
                        if !descarte_valido && outra["elegivel"] != true { regra["elegivel"] = Value::Bool(false); }
                        if usar_outra { *regra = outra.clone(); regra["sem_descarte"] = Value::Bool(true); }
                    }
                }
                Some((d, a))
            }
            (a, b) => a.or(b),
        }
    };
    let elegivel = |resultado: &Value, id: &str| {
        resultado["data"]["native_result"]["transicoes"]["regras"]
            .as_array()
            .is_some_and(|regras| regras.iter().any(|r| r["id"] == id && r["elegivel"] == true))
    };
    let mut achadas = Vec::new();
    let mut anterior = dib;
    for meses in 1..=LIMITE_MESES {
        let data = somar_meses(dib, meses)?;
        let Some((_, resultado)) = simular(data) else { anterior = data; continue };
        let novas: Vec<String> = pendentes.iter().filter(|id| elegivel(&resultado, id)).cloned().collect();
        for id in novas {
            // Busca binária do primeiro dia elegível entre a data anterior e esta.
            let (mut baixo, mut alto) = (anterior, data);
            let mut melhor = (data.format("%Y-%m-%d").to_string(), resultado.clone());
            while (alto - baixo).num_days() > 1 {
                let meio = baixo + chrono::Duration::days((alto - baixo).num_days() / 2);
                match simular(meio) {
                    Some((dib_meio, r)) if elegivel(&r, &id) => { alto = meio; melhor = (dib_meio, r); }
                    _ => baixo = meio,
                }
            }
            let (data_texto, r) = melhor;
            let t = &r["data"]["native_result"]["transicoes"];
            let regra = t["regras"].as_array().and_then(|rs| rs.iter().find(|x| x["id"] == id.as_str())).cloned().unwrap_or(Value::Null);
            pendentes.retain(|p| p != &id);
            achadas.push(serde_json::json!({
                "id": id,
                "nome": regra["nome"],
                "fundamento": regra["fundamento"],
                "data": data_texto,
                "rmi": regra["rmi_final"],
                "tempo_total_meses": t["tempo_total_meses"],
                "idade_meses": t["idade_meses"],
                "carencia_meses": r["data"]["native_result"]["resumo"]["carencia_meses"],
                "descarte_aplicado": regra["sem_descarte"] != true,
            }));
        }
        anterior = data;
        if pendentes.is_empty() {
            break;
        }
    }
    achadas.sort_by(|a, b| a["data"].as_str().cmp(&b["data"].as_str()));
    Some(serde_json::json!({
        "premissa": format!("Contribuição contínua com o último salário conferido (competência {}, R$ {}), sem reajuste real, a partir de {}.", ultimo.competence, ultimo.amount, fim_vinculos.format("%d/%m/%Y")),
        "limite_meses": LIMITE_MESES,
        "regras": achadas,
        "sem_projecao": pendentes,
    }))
}
