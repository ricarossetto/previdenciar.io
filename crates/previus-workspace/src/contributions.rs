//! Reviewed contribution overlays never modify imported CNIS source records.
use crate::{Calculation, ManualSalary};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionRecord {
    pub key: String,
    pub source_indices: Vec<usize>,
    pub competence: String,
    pub amount: String,
    pub currency: String,
    pub link_sequence: Option<u32>,
    pub indicators: String,
    pub confirmed: bool,
    pub excluded: bool,
    pub evidence: String,
    /// Origem do valor no CNIS. Ausente em bases antigas: remuneração.
    #[serde(default = "default_amount_kind")]
    pub amount_kind: previus_cnis::AmountKind,
}
fn default_amount_kind() -> previus_cnis::AmountKind {
    previus_cnis::AmountKind::Remuneration
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionChange {
    pub sequence: usize,
    pub action: String,
    pub reason: String,
    pub at: String,
    pub before: Vec<ContributionRecord>,
    pub after: Vec<ContributionRecord>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionReview {
    pub version: u32,
    pub source_sha256: Option<String>,
    pub records: Vec<ContributionRecord>,
    pub history: Vec<ContributionChange>,
    pub applied: bool,
}
pub fn amount_cents(value: &str) -> Result<i64, String> {
    let (whole, fraction) = value
        .split_once('.')
        .ok_or("Valor de contribuição deve usar centavos exatos.")?;
    if whole.is_empty()
        || whole.len() > 17
        || (whole.len() > 1 && whole.starts_with('0'))
        || fraction.len() != 2
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("Valor de contribuição não canônico.".into());
    }
    whole
        .parse::<i64>()
        .ok()
        .and_then(|v| v.checked_mul(100))
        .and_then(|v| fraction.parse::<i64>().ok().and_then(|f| v.checked_add(f)))
        .ok_or_else(|| "Valor de contribuição excede o limite de centavos.".into())
}
fn check_record(
    record: &ContributionRecord,
    source_ids: &HashSet<usize>,
    link_ids: &HashSet<u32>,
) -> Result<(), String> {
    if record.key.is_empty()
        || record.key.len() > 100
        || !record
            .key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b":._-".contains(&b))
    {
        return Err("Chave de contribuição inválida.".into());
    }
    let month = record.competence.as_bytes();
    if month.len() != 7
        || month[4] != b'-'
        || !month[..4].iter().all(u8::is_ascii_digit)
        || !month[5..].iter().all(u8::is_ascii_digit)
        || &record.competence[..4] == "0000"
        || !matches!(record.competence[5..].parse::<u32>(), Ok(1..=12))
    {
        return Err("Competência de contribuição inválida.".into());
    }
    amount_cents(&record.amount)?;
    if !matches!(
        record.currency.as_str(),
        "unconfirmed" | "BRL" | "CRUZEIRO" | "CRUZADO" | "URV" | "OTHER"
    ) {
        return Err("Moeda de contribuição inválida.".into());
    }
    if record.indicators.chars().count() > 1000
        || record.evidence.chars().count() > 1000
        || ((record.confirmed || record.excluded) && record.evidence.trim().is_empty())
    {
        return Err("Confirmação/exclusão exige justificativa dentro do limite.".into());
    }
    if let Some(link) = record.link_sequence {
        if !link_ids.contains(&link) {
            return Err("Vínculo de contribuição não encontrado na origem.".into());
        }
    }
    if record.source_indices.len() > 10000 {
        return Err("Origens de contribuição excedem o limite.".into());
    }
    let mut refs = HashSet::new();
    for index in &record.source_indices {
        if !refs.insert(*index) || !source_ids.contains(index) {
            return Err("Origem de contribuição ausente ou duplicada.".into());
        }
    }
    Ok(())
}
impl ContributionReview {
    pub fn validate(&self, case: &Calculation) -> Result<(), String> {
        if self.version != 1 || self.records.len() > 10000 || self.history.len() > 200 {
            return Err(
                "Versão ou quantidade de registros do livro de contribuições inválida.".into(),
            );
        }
        if self.source_sha256
            != case
                .cnis
                .as_ref()
                .and_then(|cnis| cnis.metadata.source_sha256.clone())
        {
            return Err("Livro de contribuições não corresponde ao documento de origem.".into());
        }
        let source_ids: HashSet<usize> = case
            .cnis
            .as_ref()
            .map(|cnis| cnis.salary_entries.iter().map(|e| e.source.index).collect())
            .unwrap_or_default();
        let link_ids: HashSet<u32> = case
            .cnis
            .as_ref()
            .map(|cnis| cnis.links.iter().map(|l| l.sequence).collect())
            .unwrap_or_default();
        if case
            .cnis
            .as_ref()
            .is_some_and(|cnis| source_ids.len() != cnis.salary_entries.len())
        {
            return Err("Origem contém índices duplicados.".into());
        }
        let mut keys = HashSet::new();
        let mut sources = HashSet::new();
        for record in &self.records {
            check_record(record, &source_ids, &link_ids)?;
            if !keys.insert(&record.key) {
                return Err("Contribuição com chave duplicada.".into());
            }
            for index in &record.source_indices {
                if !sources.insert(*index) {
                    return Err("Origem contabilizada em mais de uma contribuição.".into());
                }
            }
        }
        if case.cnis.as_ref().is_some_and(|cnis| {
            cnis.salary_entries.len() != sources.len()
                || cnis
                    .salary_entries
                    .iter()
                    .any(|e| !sources.contains(&e.source.index))
        }) {
            return Err(
                "Livro omite lançamentos da origem; exclua da hipótese sem apagar a fonte.".into(),
            );
        }
        let mut audit_rows = 0usize;
        for (index, change) in self.history.iter().enumerate() {
            if change.sequence != index + 1
                || !matches!(
                    change.action.as_str(),
                    "edit"
                        | "add"
                        | "confirm"
                        | "exclude"
                        | "restore"
                        | "merge"
                        | "undo"
                        | "currency"
                        | "apply"
                )
                || change.reason.trim().is_empty()
                || change.reason.chars().count() > 1000
                || change.at.len() > 64
                || chrono::DateTime::parse_from_rfc3339(&change.at).is_err()
            {
                return Err("Operação de revisão inválida ou sem justificativa.".into());
            }
            audit_rows = audit_rows
                .checked_add(change.before.len() + change.after.len())
                .ok_or("Histórico excede limite.")?;
            if audit_rows > 50000 {
                return Err("Histórico de contribuições excede limite.".into());
            }
            for side in [&change.before, &change.after] {
                let mut seen = HashSet::new();
                for record in side {
                    check_record(record, &source_ids, &link_ids)?;
                    if !seen.insert(&record.key) {
                        return Err("Histórico contém chave duplicada.".into());
                    }
                }
            }
        }
        // Reverse each diff to verify chain continuity, without trusting client flags.
        let mut state: HashMap<String, ContributionRecord> = self
            .records
            .iter()
            .cloned()
            .map(|r| (r.key.clone(), r))
            .collect();
        for change in self.history.iter().rev() {
            let affected: HashSet<_> = change
                .before
                .iter()
                .chain(change.after.iter())
                .map(|r| r.key.clone())
                .collect();
            for key in &affected {
                let expected = change.after.iter().find(|r| &r.key == key);
                if state.get(key) != expected {
                    return Err("Histórico não corresponde ao estado das contribuições.".into());
                }
            }
            for key in affected {
                state.remove(&key);
            }
            for record in &change.before {
                state.insert(record.key.clone(), record.clone());
            }
        }
        if let Some(cnis) = &case.cnis {
            for entry in &cnis.salary_entries {
                let key = format!("source:{}", entry.source.index);
                let record = state
                    .get(&key)
                    .ok_or("Preparação inicial omite origem de contribuição.")?;
                if record.source_indices != vec![entry.source.index]
                    || record.competence != entry.competence
                    || amount_cents(&record.amount)? != entry.amount_centavos
                    || record.link_sequence != entry.link_sequence
                    || record.currency != "unconfirmed"
                    || record.confirmed
                    || record.excluded
                    || !record.evidence.is_empty()
                {
                    return Err("Estado inicial alterou origem sem operação documentada.".into());
                }
            }
        }
        for record in state.values() {
            if record.source_indices.is_empty() {
                if record.currency != "unconfirmed"
                    || record.confirmed
                    || record.excluded
                    || !record.evidence.is_empty()
                {
                    return Err("Preparação manual inicial exige revisão separada.".into());
                }
            } else if record.source_indices.len() != 1
                || record.key != format!("source:{}", record.source_indices[0])
            {
                return Err("Origem consolidada exige operação rastreável.".into());
            }
        }
        if let Some(cnis) = &case.cnis {
            for entry in &cnis.salary_entries {
                let initial = state
                    .get(&format!("source:{}", entry.source.index))
                    .ok_or("Origem inicial ausente.")?;
                let mut indicators = entry.indicators.clone();
                let prpps = cnis
                    .links
                    .iter()
                    .find(|l| Some(l.sequence) == entry.link_sequence)
                    .is_some_and(|l| l.prpps);
                if prpps && !indicators.iter().any(|v| v == "PRPPS") {
                    indicators.push("PRPPS".into());
                }
                if entry.excluded_from_validated_domain
                    && !prpps
                    && !indicators.iter().any(|v| v == "origem_excluida_a_revisar")
                {
                    indicators.push("origem_excluida_a_revisar".into());
                }
                let mut seen = HashSet::new();
                indicators.retain(|v| seen.insert(v.clone()));
                if initial.indicators != indicators.join(", ") {
                    return Err(
                        "Indicadores originais foram alterados sem operação de revisão.".into(),
                    );
                }
            }
        }
        if self.applied {
            self.validate_projection(&case.manual_salaries)?;
        }
        Ok(())
    }
    pub fn projection(&self) -> Result<Vec<ManualSalary>, String> {
        let mut months = HashSet::new();
        let mut projected = vec![];
        for record in self.records.iter().filter(|r| !r.excluded) {
            amount_cents(&record.amount)?;
            if !matches!(record.amount_kind, previus_cnis::AmountKind::Remuneration) {
                return Err(
                    "Valor de contribuição ou consolidado sem origem salarial não pode ser aplicado como salário; confira a coluna de remuneração na origem."
                        .into(),
                );
            }
            if !record.confirmed
                || record.evidence.trim().is_empty()
                || record.currency != "BRL"
                || record.competence.as_str() < "1994-07"
                || !record.indicators.trim().is_empty()
            {
                return Err("Contribuições ativas exigem BRL, revisão, justificativa e indicadores resolvidos.".into());
            }
            if !months.insert(&record.competence) {
                return Err("Concomitância exige consolidação documental por competência.".into());
            }
            projected.push(ManualSalary {
                id: 0,
                competence: record.competence.clone(),
                amount: record.amount.clone(),
                indicators: record.indicators.clone(),
                confirmed: true,
            });
        }
        if projected.is_empty() {
            return Err("Livro não possui contribuições ativas.".into());
        }
        Ok(projected)
    }
    pub fn validate_projection(&self, salaries: &[ManualSalary]) -> Result<(), String> {
        let expected = self.projection()?;
        if salaries.len() != expected.len()
            || salaries.iter().zip(&expected).any(|(a, b)| {
                a.competence != b.competence
                    || a.amount != b.amount
                    || a.indicators != b.indicators
                    || a.confirmed != b.confirmed
            })
        {
            return Err(
                "Salários aplicados divergem do livro de contribuições; reaplique a revisão."
                    .into(),
            );
        }
        Ok(())
    }
}
