use crate::{blockers, Calculation, Workspace};
use previus_cnis::{AmountKind, ValidationStatus};
use std::fmt::Write;

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&#39;")
}
fn row(out: &mut String, cells: &[String]) {
    out.push_str("<tr>");
    for cell in cells { let _ = write!(out, "<td>{}</td>", escape(cell)); }
    out.push_str("</tr>\n");
}
fn table(out: &mut String, caption: &str, headings: &[&str]) {
    let _ = write!(out, "<table><caption>{}</caption><thead><tr>", escape(caption));
    for title in headings { let _ = write!(out, "<th scope=\"col\">{}</th>", escape(title)); }
    out.push_str("</tr></thead><tbody>\n");
}
fn end_table(out: &mut String) { out.push_str("</tbody></table>\n"); }
fn label(status: &ValidationStatus) -> &'static str {
    match status { ValidationStatus::Unvalidated => "Não validado", ValidationStatus::UnresolvedPrpps => "PRPPS não resolvido" }
}
fn amount_kind(kind: &AmountKind) -> &'static str {
    match kind { AmountKind::Remuneration => "remuneration", AmountKind::ContributionColumnUnresolved => "contribution_column_unresolved", AmountKind::ConsolidatedAnnualValue => "consolidated_annual_value" }
}
fn nominal(value: i64) -> String {
    let unsigned = value.unsigned_abs();
    format!("{}{}.{:02}", if value < 0 { "-" } else { "" }, unsigned / 100, unsigned % 100)
}

/// Standalone HTML with no scripts/external resources. All user/extracted text
/// is escaped. include_personal=false suppresses direct identity, employers,
/// free-text titles/notes/settings and imported free-text warning messages.
/// Dates and monetary records remain sensitive: this is NOT full anonymization.
/// result is intentionally not rendered: arbitrary stored JSON cannot become
/// a claimed RMI, entitlement or verified financial answer.
pub fn report_html(case: &Calculation, workspace: &Workspace, include_personal: bool) -> String {
    let mut out = String::from("<!doctype html><html lang=\"pt-BR\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Revisão técnica — Previus Rust</title><style>body{font-family:system-ui,sans-serif;margin:2rem;color:#17212b}table{border-collapse:collapse;width:100%;margin:1rem 0}th,td{border:1px solid #bbc3cb;padding:.4rem;text-align:left;overflow-wrap:anywhere}caption{text-align:left;font-weight:bold}.warning{border:2px solid #955d00;padding:1rem}pre{white-space:pre-wrap}</style></head><body><h1>Revisão técnica — Previus Rust</h1><p class=\"warning\">RMI, direito a benefício, tempo de contribuição e regras financeiras: PENDENTES. Este relatório não é parecer jurídico nem cálculo de benefício validado. Confirmação manual de uma linha não valida regras previdenciárias.</p>");
    let _ = write!(out, "<p>Schema: {} · Revisão: {} · Cálculo interno: {}</p>", workspace.schema_version, workspace.revision, case.id);
    if include_personal {
        let _ = write!(out, "<h2>{}</h2><p>{}</p>", escape(&case.name), escape(&workspace.settings.office_name));
        table(&mut out, "Identidade informada (não inferida)", &["Campo", "Valor"]);
        for (name, value) in [("Nome", &case.identity.full_name), ("CPF", &case.identity.cpf),
            ("NIT", &case.identity.nit), ("Nascimento", &case.identity.birth_date), ("Nome da mãe", &case.identity.mother_name),
            ("Sexo", &case.sex), ("Deficiência", &case.disability)] {
            row(&mut out, &[name.into(), value.clone().unwrap_or_else(|| "Não informado".into())]);
        }
        end_table(&mut out);
        let _ = write!(out, "<p>Cenário: {}</p><pre>{}</pre>", escape(&case.scenario), escape(&case.notes));
    } else { out.push_str("<p>Identificação direta e textos pessoais omitidos. Datas e valores ainda são dados privados.</p>"); }
    let _ = write!(out, "<p>DIB informada: {}</p><h2>Pendências de revisão</h2><ul>", escape(&case.dib));
    for warning in blockers(case) { let _ = write!(out, "<li>{}</li>", escape(&warning)); }
    out.push_str("</ul>");
    if case.result.is_some() { out.push_str("<p>Há resultado JSON armazenado; não foi exibido nem tratado como RMI/elegibilidade verificada.</p>"); }
    if let Some(cnis) = &case.cnis {
        let _ = write!(out, "<h2>CNIS importado</h2><p>{} lançamentos · {} competências distintas · {} vínculos. Valores em centésimos nominais da unidade fonte, sem conversão monetária histórica ou teto validado.</p>", cnis.salary_entries.len(), cnis.salary_months.len(), cnis.links.len());
        table(&mut out, "Vínculos importados — dados originais preservados", &["Seq.", "Empregador", "Início", "Fim declarado", "Fim candidato (não confirmado)", "Indicadores", "Situação", "Excluído do domínio validado", "Fonte"]);
        for link in &cnis.links {
            row(&mut out, &[link.sequence.to_string(), if include_personal { link.raw_employer.clone().unwrap_or_default() } else { "Omitido".into() },
                link.start_date.clone().unwrap_or_else(|| "Ausente".into()), link.end_date.clone().unwrap_or_else(|| "Ausente".into()),
                link.inferred_last_salary_end.as_ref().map(|e| e.date_candidate.clone()).unwrap_or_default(),
                link.indicators.join("; "), label(&link.validation_status).into(), link.excluded_from_validated_domain.to_string(),
                format!("p. {} / linha {}", link.source.page, link.source.line)]);
        }
        end_table(&mut out);
        table(&mut out, "Remunerações importadas (sem consolidação normativa)", &["Competência", "Valor nominal fonte", "Vínculo", "Indicadores", "Coluna", "Situação", "Excluído do domínio validado", "Fonte"]);
        for entry in &cnis.salary_entries {
            row(&mut out, &[entry.competence.clone(), nominal(entry.amount_centavos), entry.link_sequence.map(|v| v.to_string()).unwrap_or_else(|| "Não reconhecido".into()),
                entry.indicators.join("; "), amount_kind(&entry.amount_kind).into(), label(&entry.validation_status).into(), entry.excluded_from_validated_domain.to_string(),
                format!("p. {} / linha {}", entry.source.page, entry.source.line)]);
        }
        end_table(&mut out);
        table(&mut out, "Avisos e achados do importador", &["Código", "Quantidade", "Mensagem"]);
        for finding in cnis.warnings.iter().chain(cnis.findings.iter()) {
            row(&mut out, &[finding.code.clone(), finding.count.to_string(), if include_personal { finding.message.clone() } else { "Mensagem livre omitida".into() }]);
        }
        end_table(&mut out);
        table(&mut out, "Benefícios — não considerados automaticamente como vínculos", &["Seq.", "Espécie", "Estado", "Incluído como vínculo"]);
        for benefit in &cnis.benefits {
            row(&mut out, &[benefit.sequence.to_string(), benefit.species.map(|v| v.to_string()).unwrap_or_else(|| "Desconhecida".into()),
                match benefit.status { previus_cnis::BenefitStatus::Denied => "Indeferido", previus_cnis::BenefitStatus::Unknown => "Desconhecido" }.into(), benefit.included_as_employment_link.to_string()]);
        }
        end_table(&mut out);
    }
    table(&mut out, "Vínculos manuais — não substituem pendências do CNIS", &["ID", "Empregador", "Início", "Fim", "Regime", "Indicadores", "Linha confirmada"]);
    for link in &case.manual_links {
        row(&mut out, &[link.id.to_string(), if include_personal { link.employer.clone() } else { "Omitido".into() }, link.start_date.clone(),
            link.end_date.clone(), link.regime.clone(), link.indicators.clone(), link.confirmed.to_string()]);
    }
    end_table(&mut out);
    table(&mut out, "Remunerações manuais", &["ID", "Competência", "Valor informado (unidade não validada)", "Indicadores", "Linha confirmada"]);
    for salary in &case.manual_salaries {
        row(&mut out, &[salary.id.to_string(), salary.competence.clone(), salary.amount.clone(), salary.indicators.clone(), salary.confirmed.to_string()]);
    }
    end_table(&mut out);
    out.push_str("<h2>Parâmetros informados — sem aplicação financeira</h2>");
    table(&mut out, "Opções do cálculo (não são índices/taxas validados)", &["Opção", "Valor"]);
    for (name, value) in [("Índice de correção", &case.options.correction_index), ("Taxa de juros", &case.options.interest_rate),
        ("Tipo de juros", &case.options.interest_kind), ("Honorários percentuais", &case.options.fees_percent), ("Honorários fixos", &case.options.fees_fixed),
        ("Data de prescrição", &case.options.prescription_date), ("Base de referência", &case.options.reference_base), ("Política de regras", &case.options.rule_policy)] {
        row(&mut out, &[name.into(), value.clone()]);
    }
    row(&mut out, &["Incluir décimo terceiro (não aplicado)".into(), case.options.include_thirteenth.to_string()]); end_table(&mut out);
    if include_personal { let _ = write!(out, "<footer>{}</footer>", escape(&workspace.settings.report_footer)); }
    out.push_str("</body></html>"); out
}

fn csv_cell(value: &str) -> String {
    // Quote delimiters/newlines AND neutralize spreadsheet formula starters.
    // Whitespace/BOM/zero-width before a starter is not a bypass. Tab/CR/LF
    // themselves at the beginning are also dangerous. Negative values remain
    // textual source values, not spreadsheet formulas or validated BRL.
    let first = value.chars().find(|c| !c.is_whitespace() && !matches!(c, '\u{feff}' | '\u{200b}'));
    let dangerous = matches!(first, Some('=' | '+' | '-' | '@')) || value.starts_with(['\t', '\r', '\n']);
    let escaped = value.replace('"', "\"\"");
    format!("\"{}{}\"", if dangerous { "'" } else { "" }, escaped)
}
fn csv_row(out: &mut String, cells: &[String]) {
    out.push_str(&cells.iter().map(|s| csv_cell(s)).collect::<Vec<_>>().join(",")); out.push_str("\r\n");
}

/// Source-level salary rows, not computed/merged BRL or a benefit amount.
/// Contains no identity fields or employers. It is still private financial
/// data. The caller chooses an explicit export path (never autosaved here).
pub fn export_salary_csv(case: &Calculation) -> String {
    let mut out = String::new();
    csv_row(&mut out, &["origin", "competence", "source_nominal_amount", "unit_policy", "link_sequence", "indicators", "amount_kind", "validation_status", "excluded_from_validated_domain", "confirmed_manual_row", "source_page", "source_line"].map(str::to_owned));
    if let Some(cnis) = &case.cnis {
        for entry in &cnis.salary_entries {
            csv_row(&mut out, &["cnis".into(), entry.competence.clone(), nominal(entry.amount_centavos), "currency_units_unverified".into(),
                entry.link_sequence.map(|n| n.to_string()).unwrap_or_default(), entry.indicators.join(";"), amount_kind(&entry.amount_kind).into(),
                label(&entry.validation_status).into(), entry.excluded_from_validated_domain.to_string(), String::new(), entry.source.page.to_string(), entry.source.line.to_string()]);
        }
    }
    for entry in &case.manual_salaries {
        csv_row(&mut out, &["manual".into(), entry.competence.clone(), entry.amount.clone(), "currency_units_unverified".into(), String::new(), entry.indicators.clone(), "manual_unvalidated".into(), "Não validado".into(), String::new(), entry.confirmed.to_string(), String::new(), String::new()]);
    }
    out
}
