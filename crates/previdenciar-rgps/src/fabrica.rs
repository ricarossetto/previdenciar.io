//! Optional online RMI conference at Fábrica de Cálculos/TRF3.
//!
//! The app sends only reviewed dates, sex, species and salaries. This module
//! builds the provider payload from a closed whitelist and parses a compact
//! summary; identity, CNIS, notes and the API key never enter the payload.

use chrono::Datelike;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::str::FromStr;

pub const BASE_URL: &str = "https://fabrica-de-calculos.app.trf3.jus.br";
pub const ENDPOINT: &str =
    "https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-rmi";
pub const ENDPOINT_RMI: &str = ENDPOINT;
pub const ENDPOINT_RMI_POR_FUNDAMENTO: &str =
    "https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-rmi/por-fundamento";
pub const ENDPOINT_TEMPO: &str =
    "https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-tempo";
pub const ENDPOINT_TEMPO_GRUPOS: &str =
    "https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-tempo/por-grupos";
pub const ENDPOINT_ATRASADOS: &str =
    "https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-atrasados";
pub const VALID_ESPECIES: [u16; 17] = [
    21, 25, 31, 32, 36, 41, 42, 46, 57, 80, 87, 88, 91, 92, 93, 94, 95,
];
pub const DOCUMENT_TYPES: [&str; 5] = ["cnis", "rdctc", "carta", "pap", "prevjud"];
pub const INDEX_TYPES: [&str; 9] = [
    "tabela",
    "reajuste",
    "moedas",
    "tabua-mortalidade",
    "adiantamento",
    "limites",
    "limite-baixa-renda-reclusao",
    "os121",
    "reajuste-pre-real",
];
pub const MAX_PERIODS: usize = 200;
pub const MAX_SALARIES: usize = 2400;
const MIN_DATE: &str = "1973-06-11";
const MAX_MONEY: Decimal = Decimal::from_parts(1_000_000_000, 0, 0, false, 2);

pub fn query_suffix(demonstrativo: bool, permalink: bool) -> String {
    match (demonstrativo, permalink) {
        (true, true) => "?demonstrativo=true&permalink=true".into(),
        (true, false) => "?demonstrativo=true".into(),
        (false, true) => "?permalink=true".into(),
        (false, false) => String::new(),
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FabricaInput {
    pub data_apuracao: String,
    pub data_nascimento: String,
    /// M or F. The API uses the same domain.
    pub sexo: String,
    /// All 17 RGPS/assistential species supported by TRF3.
    pub especie: u16,
    #[serde(default)]
    pub isencao_carencia: bool,
    #[serde(default)]
    pub tipo: Option<String>,
    /// DII / óbito / fato gerador; defaults to DIB when applicable.
    #[serde(default)]
    pub fato_gerador: Option<String>,
    #[serde(default)]
    pub der: Option<String>,
    #[serde(default)]
    pub fundamento: Option<String>,
    #[serde(default)]
    pub marco_temporal: Option<String>,
    #[serde(default)]
    pub aplicar_irsm_fev94: Option<bool>,
    #[serde(default)]
    pub calcular_rmi_pcd_com_regra_anterior: Option<bool>,
    #[serde(default)]
    pub aplicar_art3_lei11718: Option<bool>,
    #[serde(default)]
    pub numero_dependentes: Option<u8>,
    #[serde(default)]
    pub coeficiente_auxilio_acidente: Option<f64>,
    #[serde(default)]
    pub data_acidente: Option<String>,
    #[serde(default)]
    pub criterio_salario_maternidade: Option<String>,
    #[serde(default)]
    pub descarte_manual: Option<Vec<String>>,
    #[serde(default)]
    pub demonstrativo: bool,
    #[serde(default)]
    pub permalink: bool,
    pub periodos: Vec<FabricaPeriodo>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FabricaPeriodo {
    pub data_inicial: String,
    pub data_final: String,
    #[serde(default)]
    pub descricao: Option<String>,
    #[serde(default)]
    pub forma_contagem: Option<String>,
    #[serde(default)]
    pub natureza_vinculo: Option<String>,
    #[serde(default)]
    pub contar_carencia: Option<bool>,
    #[serde(default)]
    pub prioritario: Option<bool>,
    #[serde(default)]
    pub indicadores: Vec<String>,
    #[serde(default)]
    pub salarios: Vec<FabricaSalario>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FabricaSalario {
    pub competencia: String,
    /// Canonical decimal string (dot, two decimals). Never a binary float.
    pub valor: String,
    #[serde(default)]
    pub excluido: Option<bool>,
    #[serde(default)]
    pub indicadores: Vec<String>,
}

fn date(value: &str, label: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| format!("{label} deve estar no formato AAAA-MM-DD."))
}

fn competence(value: &str) -> Result<(i32, u32), String> {
    let text = value.trim();
    let bytes = text.as_bytes();
    if bytes.len() != 7 || bytes[4] != b'-' {
        return Err(format!("Competência {text} deve estar no formato AAAA-MM."));
    }
    let year: i32 = text[..4]
        .parse()
        .map_err(|_| format!("Competência {text} inválida."))?;
    let month: u32 = text[5..]
        .parse()
        .map_err(|_| format!("Competência {text} inválida."))?;
    if !(1..=12).contains(&month) {
        return Err(format!("Competência {text} tem mês inválido."));
    }
    Ok((year, month))
}

fn month_key(date: NaiveDate) -> (i32, u32) {
    (date.year(), date.month())
}

fn money(value: &str) -> Result<Decimal, String> {
    let text = value.trim();
    if text.is_empty() || text.contains(',') {
        return Err("Salário deve ser um decimal canônico com ponto e duas casas.".into());
    }
    let parsed = Decimal::from_str(text).map_err(|_| "Salário inválido.".to_string())?;
    if parsed <= Decimal::ZERO {
        return Err("Salário deve ser maior que zero.".into());
    }
    if parsed > MAX_MONEY {
        return Err("Salário acima do limite aceito pela conferência.".into());
    }
    Ok(parsed)
}
/// Structural validation. Supports all 17 species, COMUM/PCD/RURICOLA, and
/// concomitant/overlapping periods resolved natively by TRF3.
pub fn validate(input: &FabricaInput) -> Result<(), String> {
    validate_with_mode(input, false)
}

/// Validates payloads used for `/calculos-tempo`, where periods without salaries are valid.
pub fn validate_tempo(input: &FabricaInput) -> Result<(), String> {
    validate_with_mode(input, true)
}

fn validate_with_mode(input: &FabricaInput, allow_empty_salaries: bool) -> Result<(), String> {
    let dib = date(&input.data_apuracao, "Data de apuração")?;
    let minimo = date(MIN_DATE, "Data mínima")?;
    if dib < minimo {
        return Err("A Fábrica de Cálculos atende apuração a partir de 1973-06-11.".into());
    }
    if let Some(der) = input.der.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        date(der, "DER")?;
    }
    let nascimento = date(&input.data_nascimento, "Data de nascimento")?;
    if nascimento >= dib {
        return Err("Nascimento deve anteceder a data de apuração.".into());
    }
    if input.sexo != "M" && input.sexo != "F" {
        return Err("Sexo deve ser M ou F.".into());
    }
    if !VALID_ESPECIES.contains(&input.especie) {
        return Err(format!("Espécie {} não pertence às 17 espécies RGPS suportadas.", input.especie));
    }
    if input.isencao_carencia && !matches!(input.especie, 31 | 32 | 91 | 92) {
        return Err("Isenção de carência só se aplica a benefícios por incapacidade (31/32/91/92).".into());
    }
    if let Some(tipo) = input.tipo.as_deref() {
        if !matches!(tipo, "COMUM" | "PCD" | "RURICOLA") {
            return Err("Tipo de segurado deve ser COMUM, PCD ou RURICOLA.".into());
        }
    }
    if let Some(fato) = input.fato_gerador.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let fato = date(fato, "Fato gerador")?;
        if fato > dib {
            return Err("Fato gerador não pode ser posterior à data de apuração.".into());
        }
        if let Some(acidente) = input.data_acidente.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let acidente = date(acidente, "Data do acidente")?;
            if acidente > fato {
                return Err("Data do acidente não pode ser posterior ao fato gerador/óbito.".into());
            }
        }
    }
    if let Some(n) = input.numero_dependentes {
        if n < 1 {
            return Err("Número de dependentes deve ser ao menos 1.".into());
        }
    }
    if let Some(c) = input.coeficiente_auxilio_acidente {
        if !matches!(c, v if (v - 0.3).abs() < 1e-6 || (v - 0.4).abs() < 1e-6 || (v - 0.6).abs() < 1e-6) {
            return Err("Coeficiente do auxílio-acidente pré-1995 deve ser 0.3, 0.4 ou 0.6.".into());
        }
    }
    if let Some(m) = input.criterio_salario_maternidade.as_deref() {
        if !matches!(m, "UM_DOZE_AVOS" | "MEDIA_SIMPLES_DOZE_ULTIMOS") {
            return Err("Critério de salário-maternidade deve ser UM_DOZE_AVOS ou MEDIA_SIMPLES_DOZE_ULTIMOS.".into());
        }
    }
    if let Some(descartes) = &input.descarte_manual {
        for comp in descartes {
            competence(comp)?;
        }
    }
    if input.periodos.is_empty() {
        return Err("Informe ao menos um período contributivo.".into());
    }
    if input.periodos.len() > MAX_PERIODS {
        return Err(format!("Máximo de {MAX_PERIODS} períodos por conferência."));
    }
    let mut total = 0usize;
    for (i, periodo) in input.periodos.iter().enumerate() {
        let inicio = date(&periodo.data_inicial, &format!("Início do período {}", i + 1))?;
        let fim = date(&periodo.data_final, &format!("Fim do período {}", i + 1))?;
        if inicio > fim {
            return Err(format!("Período {} termina antes de começar.", i + 1));
        }
        if fim > dib {
            return Err(format!(
                "Período {} termina depois da data de apuração; limite o término à DIB.",
                i + 1
            ));
        }
        let mut vistas_no_periodo = std::collections::BTreeSet::new();
        for salario in &periodo.salarios {
            let (ano, mes) = competence(&salario.competencia)?;
            let comp = (ano, mes);
            if comp < month_key(inicio) || comp > month_key(fim) {
                return Err(format!(
                    "Competência {} está fora do período {}.",
                    salario.competencia,
                    i + 1
                ));
            }
            if !vistas_no_periodo.insert(comp) {
                return Err(format!(
                    "Competência {} aparece mais de uma vez no período {}.",
                    salario.competencia,
                    i + 1
                ));
            }
            money(&salario.valor)?;
            total += 1;
        }
    }
    if !allow_empty_salaries && total == 0 {
        return Err("Informe ao menos um salário revisado para o cálculo de RMI.".into());
    }
    if total > MAX_SALARIES {
        return Err(format!("Máximo de {MAX_SALARIES} salários por conferência."));
    }
    Ok(())
}
fn json_number(value: Decimal) -> Result<Value, String> {
    let text = format!("{value:.2}");
    let number = serde_json::Number::from_str(&text)
        .map_err(|_| "Salário não pôde ser convertido para número JSON.".to_string())?;
    Ok(Value::Number(number))
}

fn build_periods_json(input: &FabricaInput) -> Result<Vec<Value>, String> {
    let mut periodos = Vec::with_capacity(input.periodos.len());
    for (i, periodo) in input.periodos.iter().enumerate() {
        let salarios = periodo
            .salarios
            .iter()
            .map(|salario| {
                let valor = money(&salario.valor)?;
                let mut item = Map::new();
                item.insert("competencia".into(), json!(salario.competencia.trim()));
                item.insert("valor".into(), json_number(valor)?);
                if let Some(excluido) = salario.excluido {
                    item.insert("excluido".into(), json!(excluido));
                }
                if !salario.indicadores.is_empty() {
                    item.insert("indicadores".into(), json!(salario.indicadores));
                }
                Ok(Value::Object(item))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let desc = periodo
            .descricao
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Período {}", i + 1));
        let mut p = Map::new();
        p.insert("dataInicial".into(), json!(periodo.data_inicial.trim()));
        p.insert("dataFinal".into(), json!(periodo.data_final.trim()));
        p.insert("descricao".into(), json!(desc));
        p.insert(
            "formaContagem".into(),
            json!(periodo.forma_contagem.as_deref().unwrap_or("COMUM")),
        );
        p.insert(
            "naturezaVinculo".into(),
            json!(periodo.natureza_vinculo.as_deref().unwrap_or("EMPREGADO")),
        );
        p.insert("contarCarencia".into(), json!(periodo.contar_carencia.unwrap_or(true)));
        p.insert("prioritario".into(), json!(periodo.prioritario.unwrap_or(false)));
        if !periodo.indicadores.is_empty() {
            p.insert("indicadores".into(), json!(periodo.indicadores));
        }
        p.insert("salarios".into(), json!(salarios));
        periodos.push(Value::Object(p));
    }
    Ok(periodos)
}

fn build_common_body(input: &FabricaInput, allow_empty_salaries: bool) -> Result<Map<String, Value>, String> {
    validate_with_mode(input, allow_empty_salaries)?;
    let periodos = build_periods_json(input)?;
    let mut opcoes = Map::new();
    opcoes.insert("especie".into(), json!(input.especie));
    if matches!(input.especie, 31 | 32 | 91 | 92) && input.isencao_carencia {
        opcoes.insert("isencaoCarenciaB31ouB32".into(), json!(true));
    }
    if input.aplicar_art3_lei11718 == Some(true) {
        opcoes.insert("aplicarArt3Lei11718".into(), json!(true));
    }
    if let Some(n) = input.numero_dependentes {
        opcoes.insert("numeroDependentes".into(), json!(n));
    }
    if let Some(coef) = input.coeficiente_auxilio_acidente {
        opcoes.insert("coeficienteAuxilioAcidente".into(), json!(coef));
    }
    let tipo = input.tipo.as_deref().unwrap_or("COMUM");
    let apuracao = input.data_apuracao.trim();
    let der = input
        .der
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(apuracao);
    let mut body = Map::new();
    body.insert("dataApuracao".into(), json!(apuracao));
    body.insert("der".into(), json!(der));
    body.insert(
        "dadosSegurado".into(),
        json!({
            "nascimento": input.data_nascimento.trim(),
            "sexo": input.sexo,
            "tipo": tipo,
        }),
    );
    body.insert("opcoesContagem".into(), Value::Object(opcoes));
    body.insert("periodosContribuicao".into(), Value::Array(periodos));
    if let Some(fato) = input.fato_gerador.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        body.insert("fatoGerador".into(), json!(fato));
    } else if matches!(input.especie, 21 | 25 | 31 | 32 | 36 | 80 | 91 | 92 | 93 | 94 | 95) {
        body.insert("fatoGerador".into(), json!(apuracao));
    }
    if let Some(acidente) = input.data_acidente.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        body.insert("dataAcidente".into(), json!(acidente));
    }
    Ok(body)
}

/// Builds the provider `/calculos-rmi` (or `/por-fundamento`) body. No name, CPF, NIT, CNIS or
/// free-form note is ever copied.
pub fn build_body(input: &FabricaInput) -> Result<Value, String> {
    let mut body = build_common_body(input, false)?;
    if let Some(f) = input.fundamento.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        body.insert("fundamento".into(), json!(f));
    }
    if let Some(m) = input.marco_temporal.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        body.insert("marcoTemporal".into(), json!(m));
    }
    if let Some(v) = input.aplicar_irsm_fev94 {
        body.insert("aplicarIrsmFev94".into(), json!(v));
    }
    if let Some(v) = input.calcular_rmi_pcd_com_regra_anterior {
        body.insert("calcularRmiPcdComRegraAnterior".into(), json!(v));
    }
    if let Some(c) = input.criterio_salario_maternidade.as_deref().filter(|s| !s.is_empty()) {
        body.insert("criterioSalarioMaternidade".into(), json!(c));
    }
    if let Some(descartes) = &input.descarte_manual {
        body.insert("descarteManual".into(), json!({ "competencias": descartes }));
    }
    Ok(Value::Object(body))
}

/// Builds the provider `/calculos-tempo` body.
pub fn build_tempo_body(input: &FabricaInput) -> Result<Value, String> {
    let body = build_common_body(input, true)?;
    Ok(Value::Object(body))
}

/// Maps a workspace Calculation directly to FabricaInput (either from manual rows or CNIS source)
/// without requiring prior local-engine confirmation or policy approval.
pub fn case_to_fabrica_input(case: &previdenciar_workspace::Calculation) -> Result<FabricaInput, String> {
    let dib_date = crate::parse_dib_date(&case.dib)
        .ok_or("Informe uma DIB válida (AAAA-MM-DD) antes de enviar à Fábrica TRF3.")?;
    let dib_iso = dib_date.format("%Y-%m-%d").to_string();
    let dib_ym = &dib_iso[..7];

    let birth_raw = case
        .identity
        .birth_date
        .as_deref()
        .or_else(|| case.cnis.as_ref().and_then(|c| c.identity.birth_date.as_deref()))
        .ok_or("Informe a data de nascimento do segurado antes de enviar à Fábrica TRF3.")?;
    let birth_iso = crate::parse_dib_date(birth_raw)
        .ok_or("Data de nascimento inválida.")?
        .format("%Y-%m-%d")
        .to_string();

    let sex_raw = case
        .sex
        .as_deref()
        .or_else(|| case.cnis.as_ref().and_then(|c| c.identity.sex.as_deref()))
        .unwrap_or("")
        .trim()
        .to_lowercase();
    let sexo = match sex_raw.as_str() {
        "m" | "masculino" => "M".to_string(),
        "f" | "feminino" => "F".to_string(),
        _ => return Err("Informe o sexo cadastral (Masculino ou Feminino) para enviar à Fábrica TRF3.".into()),
    };

    let especie: u16 = case
        .especie
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<u16>().map_err(|_| format!("Espécie {s} inválida.")))
        .transpose()?
        .unwrap_or(42);
    if !VALID_ESPECIES.contains(&especie) {
        return Err(format!("Espécie {especie} não suportada pela Fábrica TRF3."));
    }

    let disability = case.disability.as_deref().unwrap_or("").trim().to_lowercase();
    let mut tipo = if matches!(disability.as_str(), "leve" | "moderada" | "grave" | "pcd") {
        "PCD".to_string()
    } else {
        "COMUM".to_string()
    };

    let mut periodos = Vec::new();
    if !case.manual_links.is_empty() {
        for (i, link) in case.manual_links.iter().enumerate() {
            if link.source_review.as_ref().is_some_and(|r| r.excluded) {
                continue;
            }
            if link.regime.trim().eq_ignore_ascii_case("PRPPS") || link.regime.trim().eq_ignore_ascii_case("RPPS") {
                continue;
            }
            let Ok(start_d) = date(&link.start_date, "início") else { continue; };
            let Ok(end_d) = date(&link.end_date, "fim") else { continue; };
            if start_d > dib_date || end_d < start_d {
                continue;
            }
            let eff_end = end_d.min(dib_date);
            let start_ym = format!("{:04}-{:02}", start_d.year(), start_d.month());
            let end_ym = format!("{:04}-{:02}", eff_end.year(), eff_end.month());
            let mut by_comp = std::collections::BTreeMap::<String, Decimal>::new();
            for s in &case.manual_salaries {
                let comp = s.competence.trim();
                if comp >= start_ym.as_str() && comp <= end_ym.as_str() && comp < dib_ym {
                    if let Ok(v) = rgps_core::money::parse_money(&s.amount, "manual_salaries") {
                        if v > Decimal::ZERO {
                            by_comp.entry(comp.to_string()).and_modify(|cur| *cur += v).or_insert(v);
                        }
                    }
                }
            }
            let salarios = by_comp
                .into_iter()
                .map(|(competencia, v)| FabricaSalario {
                    competencia,
                    valor: format!("{v:.2}"),
                    ..Default::default()
                })
                .collect();
            let ind_lower = link.indicators.to_lowercase();
            let forma = if ind_lower.contains("especial") {
                "ESPECIAL_25"
            } else if ind_lower.contains("rural") {
                if tipo == "COMUM" { tipo = "RURICOLA".into(); }
                "RURAL"
            } else {
                "COMUM"
            };
            periodos.push(FabricaPeriodo {
                data_inicial: start_d.format("%Y-%m-%d").to_string(),
                data_final: eff_end.format("%Y-%m-%d").to_string(),
                descricao: Some(format!("Período {}", i + 1)),
                forma_contagem: Some(forma.into()),
                natureza_vinculo: Some("EMPREGADO".into()),
                contar_carencia: Some(true),
                prioritario: Some(i == 0),
                indicadores: link
                    .indicators
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect(),
                salarios,
            });
        }
    } else if let Some(cnis) = &case.cnis {
        for (i, link) in cnis.links.iter().enumerate() {
            if link.prpps {
                continue;
            }
            let Some(start_str) = link.start_date.as_deref() else { continue; };
            let Ok(start_d) = date(start_str, "cnis.inicio") else { continue; };
            let end_candidate = link
                .end_date
                .clone()
                .or_else(|| link.inferred_last_salary_end.as_ref().map(|x| x.date_candidate.clone()))
                .or_else(|| {
                    cnis.salary_entries
                        .iter()
                        .filter(|e| e.link_sequence == Some(link.sequence))
                        .map(|e| e.competence.clone())
                        .max()
                        .and_then(|ym| {
                            let y: i32 = ym.get(0..4)?.parse().ok()?;
                            let m: u32 = ym.get(5..7)?.parse().ok()?;
                            let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
                            NaiveDate::from_ymd_opt(ny, nm, 1)?.pred_opt().map(|d| d.format("%Y-%m-%d").to_string())
                        })
                })
                .unwrap_or_else(|| dib_iso.clone());
            let Ok(end_d) = date(&end_candidate, "cnis.fim") else { continue; };
            if start_d > dib_date || end_d < start_d {
                continue;
            }
            let eff_end = end_d.min(dib_date);
            let start_ym = format!("{:04}-{:02}", start_d.year(), start_d.month());
            let end_ym = format!("{:04}-{:02}", eff_end.year(), eff_end.month());
            let mut by_comp = std::collections::BTreeMap::<String, i64>::new();
            for entry in &cnis.salary_entries {
                if !matches!(entry.amount_kind, previdenciar_cnis::AmountKind::Remuneration | previdenciar_cnis::AmountKind::ConsolidatedAnnualValue) {
                    continue;
                }
                let matches_seq = entry.link_sequence == Some(link.sequence)
                    || (entry.link_sequence.is_none() && entry.competence.as_str() >= start_ym.as_str() && entry.competence.as_str() <= end_ym.as_str());
                if matches_seq && entry.competence.as_str() < dib_ym && entry.amount_centavos > 0 {
                    by_comp
                        .entry(entry.competence.clone())
                        .and_modify(|sum| *sum += entry.amount_centavos)
                        .or_insert(entry.amount_centavos);
                }
            }
            let salarios = by_comp
                .into_iter()
                .map(|(competencia, cents)| FabricaSalario {
                    competencia,
                    valor: format!("{}.{:02}", cents / 100, cents % 100),
                    ..Default::default()
                })
                .collect();
            let aff = link.affiliation.as_deref().unwrap_or("").to_lowercase();
            let natureza = if aff.contains("individual") || aff.contains("aut") {
                "CONTRIBUINTE_INDIVIDUAL"
            } else if aff.contains("dom") {
                "EMPREGADO_DOMESTICO"
            } else if aff.contains("avulso") {
                "TRABALHADOR_AVULSO"
            } else if aff.contains("facultativo") {
                "SEGURADO_FACULTATIVO"
            } else if aff.contains("especial") && aff.contains("segurado") {
                "SEGURADO_ESPECIAL"
            } else {
                "EMPREGADO"
            };
            let forma = if aff.contains("rural") || aff.contains("segurado especial") {
                if tipo == "COMUM" { tipo = "RURICOLA".into(); }
                "RURAL"
            } else {
                "COMUM"
            };
            periodos.push(FabricaPeriodo {
                data_inicial: start_d.format("%Y-%m-%d").to_string(),
                data_final: eff_end.format("%Y-%m-%d").to_string(),
                descricao: Some(format!("Vínculo CNIS {}", link.sequence)),
                forma_contagem: Some(forma.into()),
                natureza_vinculo: Some(natureza.into()),
                contar_carencia: Some(true),
                prioritario: Some(i == 0),
                indicadores: link.indicators.clone(),
                salarios,
            });
        }
    }

    if periodos.is_empty() {
        return Err("Nenhum período contributivo disponível no caso para enviar à Fábrica TRF3.".into());
    }

    Ok(FabricaInput {
        data_apuracao: dib_iso,
        data_nascimento: birth_iso,
        sexo,
        especie,
        isencao_carencia: matches!(especie, 31 | 32 | 91 | 92),
        tipo: Some(tipo),
        periodos,
        ..Default::default()
    })
}
fn money_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Number(number) => Decimal::from_str(&number.to_string())
            .ok()
            .map(|value| format!("{value:.2}")),
        Value::String(text) if !text.trim().is_empty() => Decimal::from_str(text.trim())
            .ok()
            .map(|value| format!("{value:.2}")),
        _ => None,
    }
}

fn requirement(value: &Value) -> Option<Value> {
    let result = value.get("resultado")?;
    let name = value
        .get("nome")
        .or_else(|| value.get("rotulo"))
        .and_then(Value::as_str)?
        .to_string();
    Some(json!({
        "nome": name,
        "rotulo": value.get("rotulo").cloned().unwrap_or(Value::Null),
        "exigido": result.get("requisito").cloned().unwrap_or(Value::Null),
        "apurado": result.get("apurado").cloned().unwrap_or(Value::Null),
        "cumprido": result.get("cumprido").cloned().unwrap_or(Value::Null),
        "porExtenso": result.get("porExtenso").cloned().unwrap_or(Value::Null),
        "implementadoEm": result.get("implementadoEm").cloned().unwrap_or(Value::Null),
        "nota": result.get("nota").cloned().unwrap_or(Value::Null),
        "inconsistencia": result.get("inconsistencia").cloned().unwrap_or(Value::Null),
    }))
}

fn benefit(value: &Value) -> Value {
    let dados = value.get("dadosRmi").unwrap_or(&Value::Null);
    let requisitos = value
        .pointer("/dadosApuracao/naApuracao")
        .and_then(Value::as_object)
        .map(|items| items.values().filter_map(requirement).collect::<Vec<_>>())
        .unwrap_or_default();
    json!({
        "especie": value.get("especie").cloned().unwrap_or(Value::Null),
        "descricao": value.get("descricao").cloned().unwrap_or(Value::Null),
        "chaveFundamento": value.get("chaveFundamento").cloned().unwrap_or(Value::Null),
        "fundamento": value.get("fundamento").cloned().unwrap_or(Value::Null),
        "inicioVigencia": value.get("inicioVigencia").cloned().unwrap_or(Value::Null),
        "terminoVigencia": value.get("terminoVigencia").cloned().unwrap_or(Value::Null),
        "alerta": value.get("alerta").cloned().unwrap_or(Value::Null),
        "temDireito": value.pointer("/dadosApuracao/temDireito").cloned().unwrap_or(Value::Null),
        "dadosRmi": {
            "rmi": money_value(dados.get("rmi")),
            "salarioBeneficio": money_value(dados.get("salarioBeneficio")),
            "piso": money_value(dados.get("piso")),
            "teto": money_value(dados.get("teto")),
            "rmiReajustada": money_value(dados.get("rmiReajustada")),
            "coeficiente": dados.get("coeficiente").cloned().unwrap_or(Value::Null),
            "divisor": dados.get("divisor").cloned().unwrap_or(Value::Null),
            "totalCompetenciasConsideradas": dados.get("totalCompetenciasConsideradas").cloned().unwrap_or(Value::Null),
            "competenciaInicial": dados.get("competenciaInicial").cloned().unwrap_or(Value::Null),
            "competenciaFinal": dados.get("competenciaFinal").cloned().unwrap_or(Value::Null),
            "aplicouFator": dados.get("aplicouFator").cloned().unwrap_or(Value::Null),
            "indiceReposicao": dados.get("indiceReposicao").cloned().unwrap_or(Value::Null),
        },
        "requisitos": requisitos,
    })
}

/// Parses the provider envelope into a compact summary. Preserves PDF demonstrativos
/// and permalink when requested.
pub fn parse_summary(bytes: &[u8]) -> Result<Value, String> {
    let envelope = parse_pdpj_envelope(bytes)?;
    let results = envelope
        .get("result")
        .and_then(Value::as_array)
        .ok_or("A Fábrica não retornou a lista de resultados.")?;
    let first = results
        .first()
        .ok_or("A Fábrica não retornou resultado para esta hipótese.")?;
    let maior_rmi = money_value(first.get("maiorRmi"))
        .ok_or("A Fábrica não retornou a maior RMI da hipótese.")?;
    let beneficios = first
        .get("beneficiosMaisVantajosos")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(benefit).collect::<Vec<_>>())
        .unwrap_or_default();
    let linhas = first
        .get("linhasDescartes")
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);
    let mut out = json!({
        "maiorRmi": maior_rmi,
        "moeda": first.get("moeda").cloned().unwrap_or(Value::Null),
        "beneficios": beneficios,
        "conclusao": first.get("conclusao").cloned().unwrap_or(Value::Null),
        "linhasDescartes": linhas,
        "resultados": results.len(),
    });
    attach_envelope_extras(&envelope, &mut out);
    Ok(out)
}

fn parse_pdpj_envelope(bytes: &[u8]) -> Result<Value, String> {
    let envelope: Value = serde_json::from_slice(bytes)
        .map_err(|_| "A resposta da Fábrica de Cálculos não é JSON.".to_string())?;
    if envelope.get("status").and_then(Value::as_str) != Some("ok") {
        let messages = envelope
            .get("messages")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .take(3)
                    .filter_map(Value::as_str)
                    .map(|text| text.chars().take(200).collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        return Err(if messages.is_empty() {
            "A Fábrica de Cálculos recusou a conferência sem detalhar o motivo.".into()
        } else {
            format!("A Fábrica de Cálculos recusou a conferência: {messages}")
        });
    }
    Ok(envelope)
}

fn attach_envelope_extras(envelope: &Value, out: &mut Value) {
    if let Some(map) = out.as_object_mut() {
        for key in [
            "permalink",
            "demonstrativoPdf",
            "demonstrativoDescartesPdf",
            "demonstrativoAlcadaPdf",
        ] {
            if let Some(val) = envelope.get(key) {
                if !val.is_null() {
                    map.insert(key.to_string(), val.clone());
                }
            }
        }
    }
}

/// Parses `/rgps/api/v1/calculos-tempo` or `/por-grupos` PDPJ response.
pub fn parse_tempo_summary(bytes: &[u8]) -> Result<Value, String> {
    let envelope = parse_pdpj_envelope(bytes)?;
    let mut out = json!({
        "result": envelope.get("result").cloned().unwrap_or(Value::Null),
        "messages": envelope.get("messages").cloned().unwrap_or(json!([])),
    });
    attach_envelope_extras(&envelope, &mut out);
    Ok(out)
}

/// Parses `/rgps/api/v1/calculos-atrasados` PDPJ response.
pub fn parse_atrasados_summary(bytes: &[u8]) -> Result<Value, String> {
    let envelope = parse_pdpj_envelope(bytes)?;
    let mut out = json!({
        "result": envelope.get("result").cloned().unwrap_or(Value::Null),
        "messages": envelope.get("messages").cloned().unwrap_or(json!([])),
    });
    attach_envelope_extras(&envelope, &mut out);
    Ok(out)
}

/// Parses `/rgps/api/v1/documentos/*/ler` PDPJ response.
pub fn parse_documento_response(bytes: &[u8]) -> Result<Value, String> {
    let envelope = parse_pdpj_envelope(bytes)?;
    Ok(json!({
        "result": envelope.get("result").cloned().unwrap_or(Value::Null),
        "messages": envelope.get("messages").cloned().unwrap_or(json!([])),
    }))
}
#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> FabricaInput {
        FabricaInput {
            data_apuracao: "2026-01-10".into(),
            data_nascimento: "1956-01-10".into(),
            sexo: "M".into(),
            especie: 41,
            isencao_carencia: false,
            tipo: None,
            fato_gerador: None,
            periodos: vec![FabricaPeriodo {
                data_inicial: "1994-07-01".into(),
                data_final: "2019-06-30".into(),
                salarios: vec![
                    FabricaSalario {
                        competencia: "1994-07".into(),
                        valor: "2000.00".into(),
                        ..Default::default()
                    },
                    FabricaSalario {
                        competencia: "1994-08".into(),
                        valor: "2000.00".into(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn body_contains_only_the_whitelist_and_generic_descriptions() {
        let body = build_body(&input()).unwrap();
        assert_eq!(body["dadosSegurado"].as_object().unwrap().len(), 3);
        assert!(body["dadosSegurado"].get("nome").is_none());
        assert!(body.get("nome").is_none());
        assert_eq!(body["periodosContribuicao"][0]["descricao"], "Período 1");
        assert!(body["periodosContribuicao"][0]["salarios"][0]["valor"].is_number());
        assert_eq!(body["opcoesContagem"]["especie"], 41);
        assert!(body.get("fatoGerador").is_none());
    }

    #[test]
    fn extra_identity_field_is_rejected_by_the_typed_input() {
        let mut value = serde_json::to_value(input()).unwrap();
        value["nome"] = json!("Pessoa Fictícia");
        assert!(serde_json::from_value::<FabricaInput>(value).is_err());
    }

    #[test]
    fn incapacity_gets_fato_gerador_and_optional_exemption() {
        let mut value = input();
        value.especie = 32;
        value.isencao_carencia = true;
        value.fato_gerador = Some("2025-01-10".into());
        let body = build_body(&value).unwrap();
        assert_eq!(body["fatoGerador"], "2025-01-10");
        assert_eq!(body["opcoesContagem"]["isencaoCarenciaB31ouB32"], true);
    }

    #[test]
    fn invalid_inputs_fail_closed() {
        let mut sex = input();
        sex.sexo = "X".into();
        assert!(validate(&sex).is_err());

        let mut species = input();
        species.especie = 999;
        assert!(validate(&species).is_err());

        let mut exemption = input();
        exemption.isencao_carencia = true;
        assert!(validate(&exemption).is_err());

        let mut zero = input();
        zero.periodos[0].salarios[0].valor = "0.00".into();
        assert!(validate(&zero).is_err());

        let mut duplicate = input();
        duplicate.periodos[0].salarios[1].competencia = "1994-07".into();
        assert!(validate(&duplicate).is_err());

        let mut after = input();
        after.periodos[0].data_final = "2026-02-01".into();
        assert!(validate(&after).is_err());
    }

    #[test]
    fn supports_all_17_species_pcd_and_concomitant_overlapping_periods() {
        for esp in VALID_ESPECIES {
            let mut v = input();
            v.especie = esp;
            assert!(validate(&v).is_ok(), "especie {esp} should validate");
        }
        let mut pcd = input();
        pcd.tipo = Some("PCD".into());
        assert_eq!(build_body(&pcd).unwrap()["dadosSegurado"]["tipo"], "PCD");

        let mut overlap = input();
        overlap.periodos.push(FabricaPeriodo {
            data_inicial: "2019-01-01".into(),
            data_final: "2019-12-31".into(),
            salarios: vec![FabricaSalario {
                competencia: "2019-12".into(),
                valor: "2000.00".into(),
                ..Default::default()
            }],
            ..Default::default()
        });
        assert!(validate(&overlap).is_ok());
    }

    #[test]
    fn rural_type_is_preserved() {
        let mut value = input();
        value.tipo = Some("RURICOLA".into());
        let body = build_body(&value).unwrap();
        assert_eq!(body["dadosSegurado"]["tipo"], "RURICOLA");
    }
    fn response() -> Vec<u8> {
        serde_json::to_vec(&json!({
            "status": "ok",
            "code": "200",
            "messages": [],
            "result": [{
                "marcoTemporal": "NA_DIB",
                "maiorRmi": 1234.56,
                "moeda": {"nome": "Real", "simbolo": "R$", "inicio": "1994-07"},
                "linhasDescartes": [{"maiorRmi": 1234.56}],
                "beneficiosMaisVantajosos": [{
                    "especie": 41,
                    "descricao": "Aposentadoria por Idade",
                    "chaveFundamento": "EC_103_ART_18",
                    "fundamento": "EC 103, art. 18",
                    "inicioVigencia": "2019-11-13",
                    "dadosApuracao": {
                        "temDireito": true,
                        "naApuracao": {
                            "idade": {
                                "nome": "Idade",
                                "rotulo": "idade",
                                "resultado": {
                                    "requisito": 23400,
                                    "apurado": 25200,
                                    "cumprido": true,
                                    "porExtenso": "cumpriu o requisito idade, com 70 anos"
                                }
                            }
                        }
                    },
                    "dadosRmi": {
                        "rmi": 1234.56,
                        "salarioBeneficio": 2057.60,
                        "coeficiente": 0.6,
                        "divisor": 300,
                        "totalCompetenciasConsideradas": 300,
                        "competenciaInicial": "1994-07",
                        "competenciaFinal": "2019-06"
                    }
                }],
                "conclusao": {"completo": true}
            }]
        }))
        .unwrap()
    }

    #[test]
    fn summary_keeps_rmi_rule_and_requirements_without_the_raw_tree() {
        let summary = parse_summary(&response()).unwrap();
        assert_eq!(summary["maiorRmi"], "1234.56");
        assert_eq!(summary["moeda"]["simbolo"], "R$");
        assert_eq!(summary["beneficios"][0]["chaveFundamento"], "EC_103_ART_18");
        assert_eq!(summary["beneficios"][0]["dadosRmi"]["rmi"], "1234.56");
        assert_eq!(
            summary["beneficios"][0]["dadosRmi"]["salarioBeneficio"],
            "2057.60"
        );
        assert_eq!(summary["beneficios"][0]["dadosRmi"]["coeficiente"], 0.6);
        let requisitos = summary["beneficios"][0]["requisitos"].as_array().unwrap();
        assert!(requisitos.iter().any(|item| item["porExtenso"]
            .as_str()
            .is_some_and(|text| text.contains("70 anos"))));
        assert!(summary.get("pbc").is_none());
        assert!(summary["beneficios"][0].get("dadosApuracao").is_none());
    }

    #[test]
    fn provider_errors_and_empty_results_are_not_a_financial_result() {
        let error = serde_json::to_vec(&json!({
            "status": "error", "code": "422", "messages": ["Entrada inválida"], "result": null
        }))
        .unwrap();
        assert!(parse_summary(&error)
            .unwrap_err()
            .contains("Entrada inválida"));
        let empty = serde_json::to_vec(&json!({"status":"ok","result":[]})).unwrap();
        assert!(parse_summary(&empty).is_err());
        let no_benefit = serde_json::to_vec(&json!({
            "status":"ok","result":[{"maiorRmi":0,"beneficiosMaisVantajosos":[],"conclusao":{"completo":true}}]
        }))
        .unwrap();
        let summary = parse_summary(&no_benefit).unwrap();
        assert_eq!(summary["maiorRmi"], "0.00");
        assert!(summary["beneficios"].as_array().unwrap().is_empty());
    }

}




