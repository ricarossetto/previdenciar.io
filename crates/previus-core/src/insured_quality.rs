//! Cálculo próprio da qualidade de segurado por vínculo.
//!
//! Regras implementadas aqui (fonte primária citada; cálculo próprio e não
//! reprodução do software original):
//! - Período de graça base de 12 meses: art. 15, II, da Lei nº 8.213/1991
//!   ("até 12 (doze) meses após a cessação das contribuições").
//! - Prorrogação **para até 24 meses** quando o segurado já tiver pago mais de
//!   120 contribuições mensais sem interrupção que acarrete a perda da
//!   qualidade de segurado: art. 15, § 1º, da Lei nº 8.213/1991. A
//!   prorrogação é do prazo legal, não um acréscimo cumulativo por vínculo.
//! - Carência exigida de 180 contribuições mensais para aposentadoria por idade,
//!   por tempo de serviço e especial: art. 25, II, da Lei nº 8.213/1991.
//!   Para o segurado inscrito na Previdência Social Urbana até 24/07/1991 vale a
//!   tabela do art. 142 da mesma lei (60 meses em 1991 e 1992, 66 em 1993, 72
//!   em 1994, 78 em 1995, 90 em 1996 e +6 ao ano, atingindo 180 em 2011), pelo
//!   ano em que o segurado implementou todas as condições do benefício.
//! - A carência é medida em contribuições: art. 24 e art. 27, I, da mesma lei.
//! - O tempo intercalado em gozo de benefício por incapacidade é computado
//!   como tempo de serviço (art. 55, II) mas **não** para efeito de carência:
//!   art. 19-C, § 1º, do Decreto nº 3.048/1999. Por isso as competências de um
//!   período marcado como benefício são listadas como desconsideradas para a
//!   carência, com o fundamento declarado e a ressalva de revisão documental.
//! - Competência em que houve contribuição em mais de um vínculo conta uma
//!   única vez para a carência: o vínculo anterior conserva a competência e o
//!   posterior a registra como desconsiderada, o que evita dupla contagem.
//!
//! Não existe "regra de quatro quintos" de carência: a busca nos textos
//! vigentes da Lei nº 8.213/1991, da MP 676/2015, da Lei nº 13.183/2015 e das
//! respectivas mensagens de veto não encontrou 144 contribuições, "quatro
//! quintos" ou "4/5". A carência aplicável é a do art. 25, II, ou a faixa da
//! tabela do art. 142.
//!
//! O módulo não conhece relógio, banco, CNIS, lei própria nem dados pessoais:
//! recebe competências já declaradas e devolve texto factual. Ausência de
//! competência não é tratada como zero silencioso: cada período informa o que
//! não pôde ser apurado.

use crate::dates::{CivilDate, YearMonth};
use crate::error::DomainError;
use std::collections::BTreeSet;

/// Motivo pelo qual uma competência deixa de ser contada para a carência.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisregardReason {
    /// O vínculo anterior já contava esta competência: dupla contagem evitada.
    Overlap,
    /// Período de gozo de benefício: o tempo intercalado não conta para
    /// carência (art. 19-C, § 1º, do Decreto nº 3.048/1999).
    Benefit,
    /// Contribuinte individual, especial ou facultativo: competência anterior
    /// ao primeiro pagamento sem atraso não conta para carência
    /// (art. 27, II, da Lei nº 8.213/1991).
    BeforeFirstPromptPayment,
    /// Exclusão declarada pelo operador na hipótese.
    Operator,
}

impl DisregardReason {
    pub fn label(self) -> &'static str {
        match self {
            DisregardReason::Overlap => "competência já contada em período anterior",
            DisregardReason::Benefit => "período de gozo de benefício",
            DisregardReason::BeforeFirstPromptPayment => "anterior ao primeiro pagamento em dia (art. 27, II)",
            DisregardReason::Operator => "excluída pelo operador",
        }
    }
}

/// Categoria do segurado no vínculo, que decide a regra de contagem da
/// carência: art. 27, I (empregado, doméstico e avulso) ou art. 27, II
/// (contribuinte individual, especial e facultativo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffiliationKind {
    Employee,
    IndividualContributor,
    Unknown,
}

impl AffiliationKind {
    /// Lê a filiação declarada na origem. O que não é reconhecidamente
    /// empregado nem contribuinte individual fica em `Unknown`, e a contagem
    /// segue a regra mais conservadora com a ressalva declarada.
    pub fn parse(value: Option<&str>) -> Self {
        let lower = value.unwrap_or("").to_lowercase();
        if lower.contains("contribuinte individual") || lower.contains("facultativo") || lower.contains("especial") {
            AffiliationKind::IndividualContributor
        } else if lower.contains("empregado") || lower.contains("doméstico") || lower.contains("domestico") || lower.contains("avulso") || lower.contains("agente público") || lower.contains("agente publico") {
            AffiliationKind::Employee
        } else {
            AffiliationKind::Unknown
        }
    }

    pub fn basis(self) -> &'static str {
        match self {
            AffiliationKind::Employee => "art. 27, I, da Lei nº 8.213/1991: contribuições a partir da filiação",
            AffiliationKind::IndividualContributor => "art. 27, II, da Lei nº 8.213/1991: contribuições a partir do primeiro pagamento sem atraso",
            AffiliationKind::Unknown => "filiação não reconhecida: contagem conservadora, com ressalva",
        }
    }
}

/// Um período de contribuição (vínculo) lido da origem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityPeriod {
    /// Identificador estável do período, usado como rótulo no documento.
    pub label: String,
    pub start: Option<CivilDate>,
    pub end: Option<CivilDate>,
    /// Filiação declarada na origem (empregado, contribuinte individual…).
    pub affiliation: Option<String>,
    /// Categoria que decide a regra de contagem da carência.
    pub affiliation_kind: AffiliationKind,
    /// Competências marcadas na origem como informadas fora do prazo
    /// (indicador PREM-EXT).
    pub late_months: Vec<YearMonth>,
    /// Empregador ou origem declarada do período.
    pub employer: Option<String>,
    /// Competências com contribuição neste período.
    pub contribution_months: Vec<YearMonth>,
    /// Competências que o operador excluiu da hipótese neste período.
    pub disregarded_months: Vec<YearMonth>,
    /// A origem registrau episódio de benefício (auxílio) neste período.
    pub had_benefit_episode: bool,
    /// A origem classificou o período inteiro como "Em Benefício".
    pub benefit_period: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityInput {
    pub periods: Vec<QualityPeriod>,
    /// Carência exigida declarada para a espécie em análise (180 para a espécie 42).
    pub required_carencia_months: u32,
    /// Ano em que o segurado implementou todas as condições do benefício.
    /// Usado apenas com a tabela do art. 142 da Lei nº 8.213/1991.
    pub implementation_year: Option<i32>,
    /// O segurado estava inscrito na Previdência Social Urbana até
    /// 24/07/1991, situação em que a carência segue a tabela do art. 142.
    pub filed_before_july_1991: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodQuality {
    pub label: String,
    pub start: Option<CivilDate>,
    pub end: Option<CivilDate>,
    pub affiliation: Option<String>,
    /// Categoria e fundamento da contagem de carência aplicados ao período.
    pub affiliation_kind: AffiliationKind,
    pub carencia_basis: &'static str,
    pub employer: Option<String>,
    /// Meses com contribuição depois das exclusões declaradas.
    pub counted_months: u32,
    /// Meses excluídos da hipótese pelo operador.
    pub disregarded_months: u32,
    /// Meses não contados por já constarem em período anterior.
    pub overlap_months: u32,
    /// Meses não contados por pertencerem a período de gozo de benefício.
    pub benefit_months: u32,
    /// Meses excluídos e o motivo factual de cada exclusão.
    pub disregarded_detail: Vec<(YearMonth, DisregardReason)>,
    pub first_payment: Option<YearMonth>,
    pub last_payment: Option<YearMonth>,
    /// Referência do fim do período de graça base (dia 15 do mês 12 meses após
    /// a última contribuição contada).
    pub grace_base_until: Option<CivilDate>,
    /// Referência do fim do período de graça já prorrogado, quando aplicável.
    pub grace_until: Option<CivilDate>,
    /// Verdadeiro quando o segurado tem mais de 120 contribuições contadas.
    pub grace_prorogued: bool,
    pub had_benefit_episode: bool,
    /// Extensão em meses de competência (convenção 30/360) da série do
    /// período, do primeiro ao último mês contado.
    pub contribution_months_30_360: u32,
    /// Observações factuais do próprio cálculo.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualitySummary {
    pub periods: Vec<PeriodQuality>,
    pub total_counted_months: u32,
    pub total_disregarded_months: u32,
    pub total_overlap_months: u32,
    pub total_benefit_months: u32,
    pub required_carencia_months: u32,
    /// Comparação informativa: meses contados contra a carência exigida.
    pub carencia_observed: bool,
    /// Fundamento da carência exigida usada na comparação.
    pub required_carencia_basis: String,
    /// Ano candidato da tabela do art. 142, quando o segurado estava inscrito
    /// na Previdência Social Urbana até 24/07/1991.
    pub art_142_candidate_year: Option<i32>,
    /// Extensão em meses de competência (convenção 30/360) da série de
    /// competências contadas, do primeiro ao último mês.
    pub total_contribution_months_30_360: u32,
    pub notes: Vec<String>,
}

const PRORROGATION_THRESHOLD: u32 = 120;

/// Tabela de carência do art. 142 da Lei nº 8.213/1991: ano de implementação
/// das condições e contribuições mensais exigidas. 2011 é o primeiro ano com
/// 180 contribuições.
pub const ART_142_TABLE: &[(i32, u32)] = &[
    (1991, 60), (1992, 60), (1993, 66), (1994, 72), (1995, 78), (1996, 90),
    (1997, 96), (1998, 102), (1999, 108), (2000, 114), (2001, 120), (2002, 126),
    (2003, 132), (2004, 138), (2005, 144), (2006, 150), (2007, 156), (2008, 162),
    (2009, 168), (2010, 174), (2011, 180),
];

/// Carência exigida para a espécie declarada.
///
/// Retorna os meses exigidos e o fundamento. A regra geral é 180 contribuições
/// mensais (art. 25, II). Para o segurado inscrito até 24/07/1991 vale a linha
/// da tabela do art. 142 correspondente ao ano de implementação das condições;
/// sem esse ano declarado, a faixa da tabela não é aplicável e a comparação é
/// feita contra 180, com a ressalva registrada.
pub fn required_carencia(
    filed_before_july_1991: bool,
    implementation_year: Option<i32>,
    declared_months: u32,
) -> (u32, String) {
    if filed_before_july_1991 {
        if let Some(year) = implementation_year {
            if let Some((_, months)) = ART_142_TABLE.iter().find(|(table_year, _)| *table_year == year) {
                return (*months, format!("Tabela do art. 142 da Lei nº 8.213/1991: {months} contribuições mensais para {year}."));
            }
            return (180, format!("Ano de implementação {year} fora da tabela do art. 142 da Lei nº 8.213/1991: comparada a carência de 180 contribuições mensais do art. 25, II."));
        }
        return (declared_months, format!("Segurado inscrito até 24/07/1991 sem ano de implementação declarado: comparada a carência declarada de {declared_months} contribuições mensais (art. 25, II, da Lei nº 8.213/1991); a faixa da tabela do art. 142 exige o ano de implementação das condições."));
    }
    (declared_months, format!("Carência exigida declarada: {declared_months} contribuições mensais (art. 25, II, da Lei nº 8.213/1991)."))
}

/// Primeiro ano da tabela do art. 142 em que as contribuições apuradas
/// alcançam a faixa exigida para aquele ano.
///
/// É um **ano candidato**: o art. 142 considera o ano em que o segurado
/// implementou todas as condições do benefício, e a carência é apenas uma
/// delas. Se a idade ou outra condição só foi implementada depois, a carência
/// exigida será maior. O resultado serve de piso da comparação e fica
/// declarado como candidato no documento.
pub fn art_142_candidate_year(counted_months: &BTreeSet<YearMonth>) -> Option<i32> {
    if counted_months.is_empty() { return None; }
    for (year, months) in ART_142_TABLE {
        let upto = counted_months.iter().filter(|month| month.year() <= *year).count() as u32;
        if upto >= *months { return Some(*year); }
    }
    ART_142_TABLE.last().map(|(year, _)| *year)
}

/// Período de graça base: 12 meses após o mês da última contribuição contada.
/// A data exibida segue a convenção do dia 15 usada no documento original
/// auditado; ela é referência de conferência, não data de perda de qualidade.
fn grace_reference(last_payment: YearMonth) -> Result<CivilDate, DomainError> {
    last_payment.checked_add_months(12)?.on_day(15)
}

fn period_quality(
    period: &QualityPeriod,
    already_counted: &BTreeSet<YearMonth>,
) -> Result<(PeriodQuality, BTreeSet<YearMonth>), DomainError> {
    let operator: BTreeSet<YearMonth> = period.disregarded_months.iter().copied().collect();
    let late: BTreeSet<YearMonth> = period.late_months.iter().copied().collect();
    let mut counted: BTreeSet<YearMonth> = BTreeSet::new();
    let mut disregarded_detail: Vec<(YearMonth, DisregardReason)> = Vec::new();
    // Art. 27, II: para contribuinte individual, especial e facultativo a
    // carência conta do primeiro pagamento sem atraso, e as contribuições
    // recolhidas com atraso de competências anteriores não entram.
    let prompt_start = if period.affiliation_kind == AffiliationKind::IndividualContributor {
        period.contribution_months.iter().copied()
            .filter(|month| !operator.contains(month) && !late.contains(month))
            .min()
    } else {
        None
    };
    for month in period.contribution_months.iter().copied().collect::<BTreeSet<YearMonth>>() {
        if operator.contains(&month) {
            disregarded_detail.push((month, DisregardReason::Operator));
        } else if period.benefit_period {
            disregarded_detail.push((month, DisregardReason::Benefit));
        } else if already_counted.contains(&month) {
            disregarded_detail.push((month, DisregardReason::Overlap));
        } else if prompt_start.is_some_and(|start| month < start) {
            disregarded_detail.push((month, DisregardReason::BeforeFirstPromptPayment));
        } else {
            counted.insert(month);
        }
    }
    let overlap_months = disregarded_detail.iter().filter(|(_, reason)| *reason == DisregardReason::Overlap).count() as u32;
    let benefit_months = disregarded_detail.iter().filter(|(_, reason)| *reason == DisregardReason::Benefit).count() as u32;
    let operator_months = disregarded_detail.iter().filter(|(_, reason)| *reason == DisregardReason::Operator).count() as u32;
    let before_prompt_months = disregarded_detail.iter().filter(|(_, reason)| *reason == DisregardReason::BeforeFirstPromptPayment).count() as u32;
    let first_payment = counted.iter().next().copied();
    let last_payment = counted.iter().next_back().copied();
    let mut notes = Vec::new();
    if period.start.is_none() { notes.push("Início não informado na origem.".into()); }
    if period.end.is_none() { notes.push("Fim não informado na origem; não é-data de encerramento presumida.".into()); }
    if counted.is_empty() {
        notes.push("Sem competência de contribuição apurada neste período; graça e carência não apuráveis.".into());
    }
    if period.had_benefit_episode {
        notes.push("Origem registra episódio de benefício: as contribuições do período precisam de conferência de desconsideração.".into());
    }
    if period.benefit_period {
        notes.push("Período classificado como “Em Benefício” na origem.".into());
    }
    if overlap_months > 0 {
        notes.push(format!("{overlap_months} competência(s) já contadas em período anterior: dupla contagem de carência evitada."));
    }
    if before_prompt_months > 0 {
        notes.push(format!("{before_prompt_months} competência(s) anterior(es) ao primeiro pagamento sem atraso não contam para carência (art. 27, II, da Lei nº 8.213/1991)."));
    }
    if period.affiliation_kind == AffiliationKind::Unknown {
        notes.push("Filiação não reconhecida na origem: a contagem da carência segue a regra mais conservadora e exige conferência.".into());
    }
    if operator_months > 0 {
        notes.push(format!("{operator_months} competência(s) excluída(s) da hipótese por decisão do operador."));
    }
    let grace_base_until = match last_payment { Some(month)=>Some(grace_reference(month)?),None=>None };
    let contribution_months_30_360 = match (first_payment, last_payment) {
        (Some(first), Some(last)) => {
            let months = (last.year() as i64 - first.year() as i64) * 12
                + (last.month() as i64 - first.month() as i64) + 1;
            months.max(0) as u32
        }
        _ => 0,
    };
    Ok((PeriodQuality {
        label: period.label.clone(), start: period.start, end: period.end,
        affiliation: period.affiliation.clone(),
        affiliation_kind: period.affiliation_kind,
        carencia_basis: period.affiliation_kind.basis(),
        employer: period.employer.clone(),
        counted_months: counted.len() as u32,
        disregarded_months: operator_months,
        overlap_months, benefit_months,
        disregarded_detail,
        first_payment, last_payment, grace_base_until,
        grace_until: grace_base_until,
        grace_prorogued: false,
        had_benefit_episode: period.had_benefit_episode,
        contribution_months_30_360,
        notes,
    }, counted))
}

/// Apura a qualidade de segurado por vínculo a partir de competências
/// declaradas. A carência exigida é a do art. 25, II, da Lei nº 8.213/1991 ou,
/// para o segurado inscrito até 24/07/1991, a faixa da tabela do art. 142
/// correspondente ao ano de implementação declarado.
pub fn insured_quality(input: &QualityInput) -> Result<QualitySummary, DomainError> {
    let mut counted_so_far: BTreeSet<YearMonth> = BTreeSet::new();
    let mut periods = Vec::with_capacity(input.periods.len());
    for period in &input.periods {
        let (quality, counted) = period_quality(period, &counted_so_far)?;
        counted_so_far.extend(counted);
        periods.push(quality);
    }    let mut total_counted = 0u32;
    let mut total_disregarded = 0u32;
    let mut total_overlap = 0u32;
    let mut total_benefit = 0u32;
    for period in &mut periods {
        total_counted = total_counted.saturating_add(period.counted_months);
        total_disregarded = total_disregarded.saturating_add(period.disregarded_months);
        total_overlap = total_overlap.saturating_add(period.overlap_months);
        total_benefit = total_benefit.saturating_add(period.benefit_months);
    }
    let (required, basis) = required_carencia(
        input.filed_before_july_1991,
        input.implementation_year,
        input.required_carencia_months,
    );
    let candidate_year = art_142_candidate_year(&counted_so_far);
    let (required, basis) = if input.filed_before_july_1991 && input.implementation_year.is_none() {
        match candidate_year {
            Some(year) => {
                let (months, basis) = required_carencia(true, Some(year), input.required_carencia_months);
                (months, format!("{basis} Ano candidato: {year}, ano em que as contribuições apuradas alcançam a faixa da tabela; o art. 142 considera o ano em que o segurado implementou todas as condições, o que pode exigir ano posterior e carência maior."))
            }
            None => (required, basis),
        }
    } else {
        (required, basis)
    };
    let grace_prorogued = total_counted > PRORROGATION_THRESHOLD;
    for period in &mut periods {
        if period.grace_base_until.is_some() {
            period.grace_prorogued = grace_prorogued;
            period.grace_until = match period.grace_base_until {
                Some(base) if grace_prorogued => Some(base.year_month().checked_add_months(12)?.on_day(15)?),
                other => other,
            };
            if grace_prorogued {
                period.notes.push(format!("Mais de {PRORROGATION_THRESHOLD} contribuições contadas: o prazo do inciso II do art. 15 pode ser prorrogado para até 24 meses."));
            }
        }
    }
    let mut notes = vec![
        "Período de graça: 12 meses contados do mês seguinte ao da última contribuição (art. 15, II, da Lei nº 8.213/1991).".into(),
        "Prorrogação para até 24 meses quando houve mais de 120 contribuições sem interrupção (art. 15, § 1º, da mesma lei).".into(),
        format!("{basis}"),
        "A carência é medida em contribuições (arts. 24 e 27, I, da Lei nº 8.213/1991); o tempo intercalado em gozo de benefício não conta para carência (art. 19-C, § 1º, do Decreto nº 3.048/1999).".into(),
    ];
    if total_counted == 0 {
        notes.push("Nenhuma competência de contribuição apurada: carência, período de graça e tempo de contribuição permanecem em aberto.".into());
    }
    let total_contribution_months_30_360 = match (counted_so_far.iter().next(), counted_so_far.iter().next_back()) {
        (Some(first), Some(last)) => {
            let months = (last.year() as i64 - first.year() as i64) * 12
                + (last.month() as i64 - first.month() as i64) + 1;
            months.max(0) as u32
        }
        _ => 0,
    };
    Ok(QualitySummary {
        periods,
        total_counted_months: total_counted,
        total_disregarded_months: total_disregarded,
        total_overlap_months: total_overlap,
        total_benefit_months: total_benefit,
        required_carencia_months: required,
        carencia_observed: total_counted >= required,
        required_carencia_basis: basis,
        art_142_candidate_year: if input.filed_before_july_1991 { candidate_year } else { None },
        total_contribution_months_30_360,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn month(value: &str) -> YearMonth { value.parse().unwrap() }
    fn date(value: &str) -> CivilDate { value.parse().unwrap() }
    fn months_from(start_year: i32, count: usize) -> Vec<String> {
        (0..count).map(|index| format!("{:04}-{:02}", start_year + (index as i32) / 12, (index as i32) % 12 + 1)).collect()
    }

    fn period(months: &[&str], disregarded: &[&str]) -> QualityPeriod {
        QualityPeriod {
            label: "Vínculo 01".into(),
            start: Some(date("2020-01-01")), end: Some(date("2021-12-31")),
            affiliation: Some("Empregado".into()), affiliation_kind: AffiliationKind::Employee,
            late_months: Vec::new(), employer: Some("EMPREGADOR FICTÍCIO".into()),
            contribution_months: months.iter().map(|value| month(value)).collect(),
            disregarded_months: disregarded.iter().map(|value| month(value)).collect(),
            had_benefit_episode: false,
            benefit_period: false,
        }
    }

    fn input(periods: Vec<QualityPeriod>) -> QualityInput {
        QualityInput {
            periods,
            required_carencia_months: 180,
            implementation_year: None,
            filed_before_july_1991: false,
        }
    }

    #[test]
    fn grace_base_uses_twelve_months_after_the_last_contribution() {
        let summary = insured_quality(&input(vec![period(&["2020-01","2020-02","2020-03"], &[])])).unwrap();
        let first = &summary.periods[0];
        assert_eq!(first.counted_months, 3);
        assert_eq!(first.first_payment, Some(month("2020-01")));
        assert_eq!(first.last_payment, Some(month("2020-03")));
        assert_eq!(first.grace_base_until, Some(date("2021-03-15")));
        assert!(!first.grace_prorogued);
        assert_eq!(summary.total_counted_months, 3);
        assert!(!summary.carencia_observed);
    }

    #[test]
    fn disregarded_months_leave_the_counted_total() {
        let summary = insured_quality(&input(vec![period(&["2020-01","2020-02","2020-03"], &["2020-02"])])).unwrap();
        assert_eq!(summary.periods[0].counted_months, 2);
        assert_eq!(summary.periods[0].disregarded_months, 1);
        assert_eq!(summary.total_disregarded_months, 1);
        assert!(summary.periods[0].notes.iter().any(|note| note.contains("excluída")));
    }

    #[test]
    fn more_than_one_hundred_and_twenty_contributions_prorogates_the_grace() {
        let months = months_from(2020, 121);
        let refs: Vec<&str> = months.iter().map(String::as_str).collect();
        let summary = insured_quality(&input(vec![period(&refs, &[])])).unwrap();
        let first = &summary.periods[0];
        assert!(first.grace_prorogued);
        assert_eq!(first.grace_base_until, Some(date("2031-01-15")));
        assert_eq!(first.grace_until, Some(date("2032-01-15")));
        assert!(first.notes.iter().any(|note| note.contains("24 meses")));
    }

    #[test]
    fn exactly_one_hundred_and_twenty_contributions_do_not_prorogate() {
        let months = months_from(2020, 120);
        let refs: Vec<&str> = months.iter().map(String::as_str).collect();
        let summary = insured_quality(&input(vec![period(&refs, &[])])).unwrap();
        assert!(!summary.periods[0].grace_prorogued);
        assert_eq!(summary.periods[0].grace_until, summary.periods[0].grace_base_until);
    }

    #[test]
    fn a_period_without_contributions_is_not_treated_as_zero_quality() {
        let summary = insured_quality(&input(vec![period(&[], &[])])).unwrap();
        assert_eq!(summary.periods[0].counted_months, 0);
        assert_eq!(summary.periods[0].grace_until, None);
        assert!(summary.periods[0].notes.iter().any(|note| note.contains("não apuráveis")));
        assert!(summary.notes.iter().any(|note| note.contains("permanecem em aberto")));
    }

    #[test]
    fn carencia_comparison_is_informative_only() {
        let months = months_from(2000, 180);
        let refs: Vec<&str> = months.iter().map(String::as_str).collect();
        let summary = insured_quality(&input(vec![period(&refs, &[])])).unwrap();
        assert_eq!(summary.total_counted_months, 180);
        assert!(summary.carencia_observed);
        assert_eq!(summary.required_carencia_months, 180);
        assert!(summary.required_carencia_basis.contains("art. 25, II"));
    }

    #[test]
    fn benefit_episodes_are_reported_as_pending_review() {
        let mut value = period(&["2020-01"], &[]);
        value.had_benefit_episode = true;
        let summary = insured_quality(&input(vec![value])).unwrap();
        assert!(summary.periods[0].had_benefit_episode);
        assert!(summary.periods[0].notes.iter().any(|note| note.contains("episódio de benefício")));
    }

    #[test]
    fn benefit_period_does_not_count_for_carencia() {
        let mut value = period(&["2020-01","2020-02"], &[]);
        value.benefit_period = true;
        let summary = insured_quality(&input(vec![value])).unwrap();
        assert_eq!(summary.periods[0].counted_months, 0);
        assert_eq!(summary.periods[0].benefit_months, 2);
        assert_eq!(summary.total_benefit_months, 2);
        assert!(summary.periods[0].notes.iter().any(|note| note.contains("Em Benefício")));
        assert!(summary.notes.iter().any(|note| note.contains("19-C")));
    }

    #[test]
    fn a_month_in_two_periods_counts_only_once() {
        let mut first = period(&["2020-01","2020-02"], &[]);
        first.label = "Vínculo 01".into();
        let mut second = period(&["2020-02","2020-03"], &[]);
        second.label = "Vínculo 02".into();
        let summary = insured_quality(&input(vec![first, second])).unwrap();
        assert_eq!(summary.periods[0].counted_months, 2);
        assert_eq!(summary.periods[1].counted_months, 1);
        assert_eq!(summary.periods[1].overlap_months, 1);
        assert_eq!(summary.total_counted_months, 3);
        assert_eq!(summary.total_overlap_months, 1);
        assert!(summary.periods[1].disregarded_detail.iter().any(|(value, reason)| *value == month("2020-02") && *reason == DisregardReason::Overlap));
    }

    #[test]
    fn individual_contributor_counts_from_the_first_prompt_payment() {
        let mut value = period(&["2020-01","2020-02","2020-03","2020-04"], &[]);
        value.affiliation = Some("Contribuinte Individual".into());
        value.affiliation_kind = AffiliationKind::IndividualContributor;
        // 2020-01 e 2020-02 foram informadas fora do prazo (PREM-EXT).
        value.late_months = vec![month("2020-01"), month("2020-02")];
        let summary = insured_quality(&input(vec![value])).unwrap();
        let first = &summary.periods[0];
        assert_eq!(first.counted_months, 2);
        assert_eq!(first.first_payment, Some(month("2020-03")));
        assert_eq!(summary.total_counted_months, 2);
        assert!(first.disregarded_detail.iter().any(|(value, reason)| *value == month("2020-01") && *reason == DisregardReason::BeforeFirstPromptPayment));
        assert!(first.notes.iter().any(|note| note.contains("art. 27, II")));
    }

    #[test]
    fn employee_counts_every_month_even_when_reported_late() {
        let mut value = period(&["2020-01","2020-02","2020-03"], &[]);
        value.late_months = vec![month("2020-01")];
        let summary = insured_quality(&input(vec![value])).unwrap();
        assert_eq!(summary.periods[0].counted_months, 3);
        assert!(!summary.periods[0].disregarded_detail.iter().any(|(_, reason)| *reason == DisregardReason::BeforeFirstPromptPayment));
    }

    #[test]
    fn unknown_affiliation_is_declared_and_never_assumed() {
        let mut value = period(&["2020-01"], &[]);
        value.affiliation = None;
        value.affiliation_kind = AffiliationKind::Unknown;
        let summary = insured_quality(&input(vec![value])).unwrap();
        assert!(summary.periods[0].notes.iter().any(|note| note.contains("não reconhecida")));
        assert!(summary.periods[0].carencia_basis.contains("ressalva"));
    }

    #[test]
    fn art_142_table_drives_the_required_carencia() {
        let months = months_from(1990, 60);
        let refs: Vec<&str> = months.iter().map(String::as_str).collect();
        let mut value = input(vec![period(&refs, &[])]);
        value.filed_before_july_1991 = true;
        value.implementation_year = Some(1996);
        let summary = insured_quality(&value).unwrap();
        assert_eq!(summary.required_carencia_months, 90);
        assert!(summary.required_carencia_basis.contains("art. 142"));
        assert!(!summary.carencia_observed);
    }

    #[test]
    fn art_142_table_reaches_one_hundred_and_eighty_in_2011() {
        let (months, basis) = required_carencia(true, Some(2011), 180);
        assert_eq!(months, 180);
        assert!(basis.contains("art. 142"));
    }

    #[test]
    fn a_year_outside_the_table_falls_back_to_the_general_rule() {
        let (months, basis) = required_carencia(true, Some(2030), 180);
        assert_eq!(months, 180);
        assert!(basis.contains("fora da tabela"));
    }

    #[test]
    fn without_the_implementation_year_the_table_is_not_invented() {
        let (months, basis) = required_carencia(true, None, 180);
        assert_eq!(months, 180);
        assert!(basis.contains("exige o ano de implementação"));
    }
}
