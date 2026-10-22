//! Independent constant-grade LC 142 scenario calculator. No IO or external service calls.
//! Source values and scenario assumptions remain distinct; this is not entitlement certification.
use crate::{blocked, issue};
use chrono::{Datelike, Duration, NaiveDate};
use previus_workspace::Calculation;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

pub const VERSION: &str = "lc142-constant-grade-v1";
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lc142Policy {
    pub version: String,
    pub calculation_date: String,
    pub correction_month: String,
    pub assumptions_confirmed: bool,
    pub review_evidence: String,
    pub disability_all_periods: bool,
    pub disability_evidence: String,
    pub rgps_confirmed: bool,
    pub project_open_links: bool,
    pub ongoing_evidence: String,
    pub fill_missing_minimum: bool,
    pub missing_salary_evidence: String,
    pub project_at_known_ceiling: bool,
    pub projection_evidence: String,
    pub historical_negative_policy: String,
    pub apply_inss_overrides: bool,
    pub snapshot: Option<Snapshot>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rate {
    pub month: String,
    pub inpc: String,
    pub igpdi: Option<String>,
    pub ipcr: Option<String>,
    pub inpc_inss: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub month: String,
    pub minimum: String,
    pub ceiling: String,
    pub minimum_source: String,
    pub ceiling_source: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub source_url: String,
    pub retrieved_at: String,
    pub response_sha256: String,
    pub through: String,
    pub rates: Vec<Rate>,
    pub limits: Vec<Limits>,
}
/// Normalize provider JSON scientific notation without binary float arithmetic.
pub fn canonical_snapshot_decimal(s: &str) -> Result<String, String> {
    if s.len() > 80 {
        return Err("Decimal público fora do limite.".into());
    }
    let value = if s.contains(['e', 'E']) {
        Decimal::from_scientific(s)
    } else {
        Decimal::from_str(s)
    }
    .map_err(|_| "Decimal público inválido.".to_owned())?;
    Ok(value.normalize().to_string())
}
fn dec(s: &str) -> Result<Decimal, String> {
    if s.len() > 40 || s.contains(['e', 'E', ',']) {
        return Err("Valor decimal inválido no snapshot ou na hipótese.".into());
    }
    Decimal::from_str(s).map_err(|_| "Valor decimal inválido no snapshot.".into())
}
fn money(v: Decimal) -> String {
    format!(
        "{:.2}",
        v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
    )
}
fn date(s: &str) -> Result<NaiveDate, String> {
    if s.len() != 10 || !s.is_ascii() {
        return Err("Data ISO inválida.".into());
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| "Data ISO inválida.".to_owned())
        .and_then(|d| {
            if (1900..=2100).contains(&d.year()) {
                Ok(d)
            } else {
                Err("Data fora do intervalo operacional 1900–2100.".into())
            }
        })
}
fn month(s: &str) -> Result<(), String> {
    if s.len() != 7
        || !s.is_ascii()
        || s.as_bytes()[4] != b'-'
        || date(&(s.to_owned() + "-01")).is_err()
    {
        return Err("Competência AAAA-MM inválida.".into());
    }
    Ok(())
}
fn next(m: &str) -> String {
    let y: i32 = m[..4].parse().unwrap();
    let mm: u32 = m[5..].parse().unwrap();
    format!(
        "{:04}-{:02}",
        y + i32::from(mm == 12),
        if mm == 12 { 1 } else { mm + 1 }
    )
}
fn period_days(a: NaiveDate, b: NaiveDate) -> i64 {
    i64::from(b.year() - a.year()) * 360
        + (i64::from(b.month()) - i64::from(a.month())) * 30
        + i64::from(b.day().min(30))
        - i64::from(a.day().min(30))
        + 1
}
fn salary_days(periods: &[(NaiveDate, NaiveDate)], competence: &str) -> i64 {
    let start = date(&(competence.to_owned() + "-01")).expect("validated competence");
    let end =
        date(&(next(competence) + "-01")).expect("bounded next competence") - Duration::days(1);
    periods
        .iter()
        .filter(|(a, b)| *a <= end && *b >= start)
        .map(|(a, b)| {
            let first = (*a).max(start).day().min(30);
            let last_date = (*b).min(end);
            let last = if last_date == end {
                30
            } else {
                last_date.day().min(30)
            };
            i64::from(last) - i64::from(first) + 1
        })
        .sum::<i64>()
        .min(30)
}
fn duration(periods: &[(NaiveDate, NaiveDate)], until: NaiveDate) -> Value {
    let days: i64 = periods
        .iter()
        .filter(|(a, _)| *a <= until)
        .map(|(a, b)| period_days(*a, (*b).min(until)))
        .sum();
    json!({"days_30_360":days,"years":days/360,"months":days%360/30,"days":days%30})
}
impl Snapshot {
    pub fn validate(&self) -> Result<(), String> {
        month(&self.through)?;
        if self.rates.is_empty()
            || self.rates.len() > 3000
            || self.limits.is_empty()
            || self.limits.len() > 3000
        {
            return Err("Snapshot vazio ou fora do limite de competências.".into());
        }
        if !self
            .source_url
            .starts_with("https://fabrica-de-calculos.app.trf3.jus.br/indices/api/v1/")
            || self.source_url.len() > 500
            || self.response_sha256.len() != 64
            || !self.response_sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || chrono::DateTime::parse_from_rfc3339(&self.retrieved_at).is_err()
        {
            return Err("Proveniência do snapshot ausente ou inválida.".into());
        }
        let mut seen = BTreeSet::new();
        for r in &self.rates {
            month(&r.month)?;
            if r.month > self.through || !seen.insert(&r.month) {
                return Err("Taxa duplicada ou posterior ao corte do snapshot.".into());
            }
            for value in [
                Some(&r.inpc),
                r.igpdi.as_ref(),
                r.ipcr.as_ref(),
                r.inpc_inss.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                let v = dec(value)?;
                if v <= Decimal::NEGATIVE_ONE || v > Decimal::from(10) {
                    return Err("Taxa fora dos limites do snapshot.".into());
                }
            }
        }
        let mut seen = BTreeSet::new();
        for r in &self.limits {
            month(&r.month)?;
            if r.month > self.through
                || !seen.insert(&r.month)
                || dec(&r.minimum)? <= Decimal::ZERO
                || dec(&r.ceiling)? < dec(&r.minimum)?
                || r.minimum_source.len() > 2000
                || r.ceiling_source.len() > 2000
            {
                return Err("Limites mensais inconsistentes ou fundamento fora do limite.".into());
            }
        }
        Ok(())
    }
}

pub fn calculate(case: &Calculation, p: &Lc142Policy) -> Result<Value, String> {
    let mut errors = Vec::new();
    if case.kind.to_lowercase() != "rgps" {
        issue(
            &mut errors,
            "lc142_kind",
            "kind",
            "Cenário LC 142 exige RGPS.",
        );
    }
    if p.version != VERSION || !p.assumptions_confirmed || p.review_evidence.trim().is_empty() {
        issue(
            &mut errors,
            "lc142_review",
            "options.rule_policy.lc142",
            "Confirme as premissas e a referência documental deste cenário.",
        );
    }
    if !p.disability_all_periods || p.disability_evidence.trim().is_empty() {
        issue(&mut errors,"lc142_disability_periods","disability","Esta versão exige um grau constante em todos os períodos, declarado e documentado. Graus mistos/conversões não são presumidos.");
    }
    if !p.rgps_confirmed {
        issue(
            &mut errors,
            "lc142_regime",
            "cnis.links",
            "Confirme documentalmente o regime dos períodos; ausência de PRPPS não comprova RGPS.",
        );
    }
    let sex = case.sex.as_deref().unwrap_or("").to_lowercase();
    let grade = case.disability.as_deref().unwrap_or("").to_lowercase();
    let required = match (sex.as_str(), grade.as_str()) {
        ("masculino", "grave") => 25,
        ("feminino", "grave") => 20,
        ("masculino", "moderada") => 29,
        ("feminino", "moderada") => 24,
        ("masculino", "leve") => 33,
        ("feminino", "leve") => 28,
        _ => {
            issue(
                &mut errors,
                "lc142_parameters",
                "identity",
                "Sexo e grau de deficiência precisam ser declarados.",
            );
            0
        }
    };
    let birth = case.identity.birth_date.as_deref().map(date).transpose()?;
    if birth.is_none() {
        issue(
            &mut errors,
            "lc142_birth",
            "identity.birth_date",
            "Confirme a data de nascimento.",
        );
    }
    let dib = date(&case.dib)?;
    let base = date(&p.calculation_date)?;
    month(&p.correction_month)?;
    if base > dib || p.correction_month != format!("{:04}-{:02}", base.year(), base.month()) {
        issue(
            &mut errors,
            "lc142_reference",
            "calculation_date",
            "A data-base deve ser anterior ou igual à DIB e corresponder ao mês da correção.",
        );
    }
    if !matches!(
        p.historical_negative_policy.as_str(),
        "preserve" | "floor_before_1999_12"
    ) {
        issue(
            &mut errors,
            "lc142_method",
            "historical_negative_policy",
            "Selecione o tratamento histórico das variações negativas.",
        );
    }
    let opts = &case.options;
    if !opts.interest_rate.is_empty()
        || !opts.interest_kind.is_empty()
        || !opts.fees_percent.is_empty()
        || !opts.fees_fixed.is_empty()
        || !opts.prescription_date.is_empty()
        || !opts.reference_base.is_empty()
        || opts.include_thirteenth
    {
        issue(&mut errors,"workspace_options_unsupported","options","Juros, honorários e atrasados são separados da RMI; parâmetros antigos não serão ignorados.");
    }
    previus_workspace::link_review::validate_source_links(case)?;
    if let Some(review) = &case.contribution_review {
        review.validate(case)?;
        if !review.applied {
            issue(
                &mut errors,
                "contribution_review_unapplied",
                "contribution_review",
                "Aplique o livro revisado antes de calcular.",
            );
        }
    }
    let Some(snapshot) = &p.snapshot else {
        issue(
            &mut errors,
            "lc142_snapshot",
            "snapshot",
            "Consulte os índices e limites públicos antes de executar.",
        );
        return Ok(blocked(errors, vec![]));
    };
    snapshot.validate()?;
    if snapshot.through != p.correction_month {
        issue(
            &mut errors,
            "lc142_snapshot_date",
            "snapshot.through",
            "Snapshot pertence a outro mês de referência.",
        );
    }
    let Some(cnis) = &case.cnis else {
        issue(
            &mut errors,
            "lc142_source",
            "cnis",
            "Esta versão do cenário requer origem CNIS preservada.",
        );
        return Ok(blocked(errors, vec![]));
    };
    if cnis
        .links
        .iter()
        .any(|l| l.prpps || l.excluded_from_validated_domain)
    {
        issue(
            &mut errors,
            "lc142_ctc",
            "cnis.links",
            "CTC/regime próprio não são convertidos automaticamente neste cenário.",
        );
    }
    if cnis.links.len() > 2000 || cnis.salary_months.len() > 2400 {
        return Err("Quantidade de vínculos/salários excede o cenário suportado.".into());
    }
    if cnis.salary_entries.iter().any(|w| {
        matches!(
            w.amount_kind,
            previus_cnis::AmountKind::ContributionColumnUnresolved
        )
    }) {
        issue(&mut errors,"lc142_salary_column","cnis.salary_entries","Há valores de contribuição sem salário identificado; não inferir salário pela alíquota.");
    }
    let consolidated_entries = cnis.salary_entries.iter().filter(|w| {
        matches!(w.amount_kind, previus_cnis::AmountKind::ConsolidatedAnnualValue)
    }).count();
    if consolidated_entries > 0 {
        issue(&mut errors,"lc142_consolidated_values","cnis.salary_entries","Há competências lidas da tabela de valores consolidados por ano civil, que não declara o vínculo de origem: a atribuição e a composição de cada mês exigem conferência documental.");
    }
    let mut intervals = vec![];
    if case.manual_links.is_empty() {
        for l in &cnis.links {
            let Some(start) = l.start_date.as_ref() else {
                issue(
                    &mut errors,
                    "lc142_link_start",
                    "cnis.links",
                    "Início de vínculo ausente.",
                );
                continue;
            };
            let end = match &l.end_date {
                Some(v) => date(v)?,
                None => {
                    if !p.project_open_links || p.ongoing_evidence.trim().is_empty() {
                        issue(&mut errors,"lc142_open_end","cnis.links","Fim ausente exige hipótese explícita de continuidade; não será substituído pelo último salário.");
                    }
                    dib
                }
            };
            let start = date(start)?;
            if start > end {
                issue(
                    &mut errors,
                    "lc142_link_dates",
                    "cnis.links",
                    "Datas de vínculo inconsistentes.",
                );
            } else if start <= dib {
                intervals.push((start, end.min(dib)))
            }
        }
    } else {
        for l in &case.manual_links {
            if l.source_review.as_ref().is_some_and(|r| r.excluded) {
                continue;
            }
            if !l.confirmed || l.regime.to_uppercase() != "RGPS" || !l.indicators.trim().is_empty()
            {
                issue(
                    &mut errors,
                    "lc142_manual_link_review",
                    "manual_links",
                    "Períodos declarados exigem RGPS, confirmação e indicadores resolvidos.",
                );
            }
            let a = date(&l.start_date)?;
            let original_end = date(&l.end_date)?;
            if a > original_end {
                issue(
                    &mut errors,
                    "lc142_link_dates",
                    "manual_links",
                    "Datas de vínculo inconsistentes.",
                );
            } else if a <= dib {
                intervals.push((a, original_end.min(dib)));
            }
        }
    }
    if let Some(birth) = birth {
        if intervals.iter().any(|(a, _)| *a < birth) {
            issue(
                &mut errors,
                "lc142_period_before_birth",
                "cnis.links",
                "Período anterior ao nascimento; corrija a origem/hipótese.",
            );
        }
    }
    intervals.sort();
    let mut merged: Vec<(NaiveDate, NaiveDate)> = vec![];
    for (a, b) in intervals {
        if let Some(last) = merged.last_mut() {
            if a <= last.1 + Duration::days(1) {
                last.1 = last.1.max(b);
                continue;
            }
        }
        merged.push((a, b));
    }
    if merged.is_empty() {
        issue(
            &mut errors,
            "lc142_no_periods",
            "cnis.links",
            "Nenhum período de contribuição declarado.",
        );
    }
    let rates: BTreeMap<_, _> = snapshot
        .rates
        .iter()
        .map(|r| (r.month.as_str(), r))
        .collect();
    let limits: BTreeMap<_, _> = snapshot
        .limits
        .iter()
        .map(|r| (r.month.as_str(), r))
        .collect();
    let Some(current_limit) = limits.get(p.correction_month.as_str()) else {
        issue(
            &mut errors,
            "lc142_limits",
            "snapshot.limits",
            "Piso/teto do mês de referência não estão disponíveis.",
        );
        return Ok(blocked(errors, vec![]));
    };
    if !errors.is_empty() {
        return Ok(blocked(errors, vec![]));
    }
    if case.contribution_review.is_none() && !case.manual_salaries.is_empty() {
        issue(&mut errors,"lc142_unreviewed_salaries","manual_salaries","Salários manuais exigem um livro de contribuições aplicado e rastreável; não serão ignorados nem substituirão a origem silenciosamente.");
        return Ok(blocked(errors, vec![]));
    }
    let mut wages: BTreeMap<String, (Decimal, &str)> = BTreeMap::new();
    if case.contribution_review.is_some() {
        for w in &case.manual_salaries {
            let amount = Decimal::from_i128_with_scale(
                previus_workspace::contributions::amount_cents(&w.amount)? as i128,
                2,
            );
            if wages
                .insert(w.competence.clone(), (amount, "reviewed_overlay"))
                .is_some()
            {
                return Err("Salário duplicado exige consolidação documental.".into());
            }
        }
    } else {
        for (m, w) in &cnis.salary_months {
            wages.insert(
                m.clone(),
                (
                    Decimal::from_i128_with_scale(w.amount_sum_centavos as i128, 2),
                    "imported",
                ),
            );
        }
    }
    let first = wages
        .keys()
        .find(|m| m.as_str() >= "1994-07")
        .ok_or("Sem salários no PBC suportado.")?
        .clone();
    let dib_month = format!("{:04}-{:02}", dib.year(), dib.month());
    let mut cursor = first;
    let mut rows = vec![];
    let mut floor_count = 0;
    let mut projection_count = 0;
    while cursor < dib_month {
        if rows.len() >= 1200 {
            return Err("PBC excede 1.200 competências.".into());
        }
        let inside = merged.iter().any(|(a, b)| {
            format!("{:04}-{:02}", a.year(), a.month()) <= cursor
                && cursor <= format!("{:04}-{:02}", b.year(), b.month())
        });
        if !inside {
            cursor = next(&cursor);
            continue;
        }
        let known_month = if cursor > p.correction_month {
            p.correction_month.as_str()
        } else {
            cursor.as_str()
        };
        let bound = limits
            .get(known_month)
            .ok_or("Snapshot sem piso/teto da competência; não extrapolar.")?;
        let (nominal, origin) = if let Some(v) = wages.get(&cursor) {
            *v
        } else if cursor < p.correction_month {
            if !p.fill_missing_minimum || p.missing_salary_evidence.trim().is_empty() {
                issue(&mut errors,"lc142_missing_salary","competencias","Há competência sem salário; escolha e documente a hipótese antes de preencher.");
                break;
            }
            floor_count += 1;
            (dec(&bound.minimum)?, "declared_missing_minimum")
        } else {
            if !p.project_at_known_ceiling || p.projection_evidence.trim().is_empty() {
                issue(
                    &mut errors,
                    "lc142_projection",
                    "projection",
                    "Salário futuro exige projeção explícita; não é contribuição observada.",
                );
                break;
            }
            projection_count += 1;
            (dec(&current_limit.ceiling)?, "declared_projection")
        };
        if nominal < Decimal::ZERO {
            return Err("Salário negativo não é suportado.".into());
        }
        let ceiling = dec(&bound.ceiling)?;
        let considered = nominal.min(ceiling);
        let mut factor = Decimal::ONE;
        let mut m = cursor.clone();
        while m < p.correction_month {
            let r = rates
                .get(m.as_str())
                .ok_or("Snapshot não cobre todas as taxas da correção.")?;
            let raw = if m.as_str() <= "1995-06" {
                r.ipcr.as_ref().ok_or("IPC-r histórico ausente.")?
            } else if m.as_str() < "1996-04" {
                &r.inpc
            } else if m.as_str() <= "2004-01" {
                r.igpdi.as_ref().ok_or("IGP-DI histórico ausente.")?
            } else if p.apply_inss_overrides {
                r.inpc_inss.as_ref().unwrap_or(&r.inpc)
            } else {
                &r.inpc
            };
            let mut rate = dec(raw)?;
            if p.historical_negative_policy == "floor_before_1999_12" && m.as_str() < "1999-12" {
                rate = rate.max(Decimal::ZERO);
            }
            factor = factor
                .checked_mul(Decimal::ONE + rate)
                .ok_or("Fator excede precisão decimal.")?;
            m = next(&m);
        }
        let corrected = considered
            .checked_mul(factor)
            .ok_or("Salário corrigido excede precisão.")?
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
        rows.push(json!({"competencia":cursor,"salario":money(nominal),"teto":money(ceiling),"considerado":money(considered),"indice":format!("{:.6}",factor.trunc_with_scale(6)),"corrigido":money(corrected),"origem":origin,"dias":salary_days(&merged,&cursor),"fonte":snapshot.source_url}));
        cursor = next(&cursor);
    }
    if !errors.is_empty() {
        return Ok(blocked(errors, vec![]));
    }
    if rows.is_empty() {
        return Err("Sem parcelas no PBC.".into());
    }
    let used = rows.len() * 80 / 100;
    if used == 0 {
        return Err("PBC insuficiente para a média de 80%.".into());
    }
    let mut ranked: Vec<_> = (0..rows.len()).collect();
    ranked.sort_by(|a, b| {
        dec(rows[*b]["corrigido"].as_str().unwrap())
            .unwrap()
            .cmp(&dec(rows[*a]["corrigido"].as_str().unwrap()).unwrap())
            .then(a.cmp(b))
    });
    let selected: BTreeSet<_> = ranked[..used].iter().copied().collect();
    let mut sum = Decimal::ZERO;
    let mut discarded = vec![];
    for (i, row) in rows.iter_mut().enumerate() {
        let chosen = selected.contains(&i);
        row["descartada"] = json!(!chosen);
        if chosen {
            sum = sum
                .checked_add(dec(row["corrigido"].as_str().unwrap())?)
                .ok_or("Soma do PBC excede precisão.")?
        } else {
            discarded.push(row["competencia"].clone());
        }
    }
    let average = (sum / Decimal::from(used))
        .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    let time = duration(&merged, dib);
    let past = duration(&merged, base);
    let mut months = BTreeSet::new();
    for (a, b) in &merged {
        let mut m = format!("{:04}-{:02}", a.year(), a.month());
        let end = format!("{:04}-{:02}", b.year(), b.month());
        while m <= end {
            months.insert(m.clone());
            m = next(&m);
        }
    }
    let eligible =
        time["days_30_360"].as_i64().unwrap_or(0) >= required * 360 && months.len() >= 180;
    let rmi = average
        .min(dec(&current_limit.ceiling)?)
        .max(dec(&current_limit.minimum)?);
    let warnings=vec!["SIMULAÇÃO com premissas declaradas, não concessão ou validação jurídica.".to_string(),"LC 142: apenas grau constante em todos os períodos; não suporta conversões por graus mistos, atividade especial, CTC automática ou fator previdenciário.".into(),format!("{floor_count} competências preenchidas por hipótese de mínimo; {projection_count} projetadas pelo teto conhecido. Não são salários extraídos."),"Períodos abertos são continuados somente por hipótese explícita. Limites futuros são congelados no snapshot conhecido, não valores oficiais futuros.".into(),"Contagem de tempo adota convenção declarada 30/360 inclusiva, com união das sobreposições; revisar a aplicação profissional.".into()];
    Ok(
        json!({"status":"experimental","legal_validation_performed":false,"rule_review_status":"declared_scenario_assumptions","warnings":warnings,"data":{"native_result":{"engine_id":VERSION,"resumo":{"competencias":rows.len(),"carencia_meses":months.len(),"soma_salarios":money(sum),"salario_beneficio":money(average),"coeficiente":"100.00","rmi_bruta":money(average),"rmi":if eligible{json!(money(rmi))}else{Value::Null},"teto":current_limit.ceiling,"piso":current_limit.minimum},"parametros":{"divisor":used,"correcao_ativa":true,"referencia_correcao":p.correction_month,"descarte_aplicado":true,"divisor_minimo_aplicado":false,"coeficiente":"100.00"},"correcao":{"referencia":p.correction_month,"pbc":rows.len(),"consideradas":used,"descartadas":discarded.len(),"parcelas":rows,"parcelas_descartadas":discarded,"soma_considerada":money(sum),"media":money(average),"salario_beneficio":money(average),"descarte_aplicado":true},"tempo":{"total":time,"sem_projecao":past,"carencia_meses":months.len(),"convention":"declared_30_360_inclusive_union","vinculos":merged.iter().map(|(a,b)|json!({"inicio":a.to_string(),"fim":b.to_string(),"dias_30_360":period_days(*a,*b)})).collect::<Vec<_>>()},"pcd":{"grau":grade,"anos_exigidos":required,"tempo_atingido_no_cenario":eligible,"calculation_date":p.calculation_date,"projection_count":projection_count,"filled_minimum_count":floor_count,"source_sha256":cnis.metadata.source_sha256,"snapshot_sha256":snapshot.response_sha256,"disability_evidence":p.disability_evidence,"review_evidence":p.review_evidence,"assumptions_confirmed":true,"actual_entitlement_verified":false},"memoria":[{"tipo":"regra","descricao":"LC 142, grau constante; média dos 80% maiores salários e coeficiente declarado de 100%","valor":money(average)}],"avisos":warnings}}}),
    )
}
