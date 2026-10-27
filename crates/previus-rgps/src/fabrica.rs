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

pub const ENDPOINT: &str =
    "https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-rmi";
pub const MAX_PERIODS: usize = 200;
pub const MAX_SALARIES: usize = 2400;
const MIN_DATE: &str = "1994-07-01";
const MAX_MONEY: Decimal = Decimal::from_parts(1_000_000_000, 0, 0, false, 2);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FabricaInput {
    pub data_apuracao: String,
    pub data_nascimento: String,
    /// M or F. The API uses the same domain.
    pub sexo: String,
    /// 31, 32, 41 or 42. Other species are not sent by this app.
    pub especie: u16,
    #[serde(default)]
    pub isencao_carencia: bool,
    #[serde(default)]
    pub tipo: Option<String>,
    /// DII for 31/32; defaults to the DIB when omitted.
    #[serde(default)]
    pub fato_gerador: Option<String>,
    pub periodos: Vec<FabricaPeriodo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FabricaPeriodo {
    pub data_inicial: String,
    pub data_final: String,
    #[serde(default)]
    pub salarios: Vec<FabricaSalario>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FabricaSalario {
    pub competencia: String,
    /// Canonical decimal string (dot, two decimals). Never a binary float.
    pub valor: String,
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
/// Structural validation. It intentionally rejects overlapping periods: the
/// online provider resolves concurrency with its own rules, and this app does
/// not want a silent divergence from the local deduplication.
pub fn validate(input: &FabricaInput) -> Result<(), String> {
    let dib = date(&input.data_apuracao, "Data de apuração")?;
    let minimo = date(MIN_DATE, "Data mínima")?;
    if dib < minimo {
        return Err("A conferência online cobre DIB a partir de 1994-07-01.".into());
    }
    let nascimento = date(&input.data_nascimento, "Data de nascimento")?;
    if nascimento >= dib {
        return Err("Nascimento deve anteceder a data de apuração.".into());
    }
    if input.sexo != "M" && input.sexo != "F" {
        return Err("Sexo deve ser M ou F.".into());
    }
    if !matches!(input.especie, 31 | 32 | 41 | 42) {
        return Err("A conferência online atende 31, 32, 41 e 42.".into());
    }
    if input.isencao_carencia && !matches!(input.especie, 31 | 32) {
        return Err("Isenção de carência só se aplica a 31/32.".into());
    }
    if let Some(tipo) = input.tipo.as_deref() {
        if tipo != "COMUM" && tipo != "RURICOLA" {
            return Err("Tipo de segurado deve ser COMUM ou RURICOLA.".into());
        }
    }
    if let Some(fato) = input.fato_gerador.as_deref() {
        let fato = date(fato, "Fato gerador")?;
        if fato > dib {
            return Err("Fato gerador não pode ser posterior à data de apuração.".into());
        }
        if matches!(input.especie, 31 | 32) && fato < minimo {
            return Err("Fato gerador anterior a 1994-07-01 não é enviado.".into());
        }
    }
    if input.periodos.is_empty() {
        return Err("Informe ao menos um período contributivo.".into());
    }
    if input.periodos.len() > MAX_PERIODS {
        return Err(format!("Máximo de {MAX_PERIODS} períodos por conferência."));
    }
    let mut ranges: Vec<(NaiveDate, NaiveDate)> = Vec::new();
    let mut total = 0usize;
    let mut vistas: std::collections::BTreeSet<(i32, u32)> = std::collections::BTreeSet::new();
    for (i, periodo) in input.periodos.iter().enumerate() {
        let inicio = date(&periodo.data_inicial, &format!("Início do período {}", i + 1))?;
        let fim = date(&periodo.data_final, &format!("Fim do período {}", i + 1))?;
        if inicio > fim {
            return Err(format!("Período {} termina antes de começar.", i + 1));
        }
        if fim > dib {
            return Err(format!(
                "Período {} termina depois da data de apuração; a conferência não projeta tempo futuro.",
                i + 1
            ));
        }
        ranges.push((inicio, fim));
        for salario in &periodo.salarios {
            let (ano, mes) = competence(&salario.competencia)?;
            let competencia = (ano, mes);
            if competencia < month_key(inicio) || competencia > month_key(fim) {
                return Err(format!(
                    "Competência {} está fora do período {}.",
                    salario.competencia,
                    i + 1
                ));
            }
            if !vistas.insert(competencia) {
                return Err(format!(
                    "Competência {} aparece mais de uma vez.",
                    salario.competencia
                ));
            }
            money(&salario.valor)?;
            total += 1;
        }
    }
    if total == 0 {
        return Err("Informe ao menos um salário revisado para a conferência.".into());
    }
    if total > MAX_SALARIES {
        return Err(format!("Máximo de {MAX_SALARIES} salários por conferência."));
    }
    ranges.sort();
    for pair in ranges.windows(2) {
        if pair[1].0 <= pair[0].1 {
            return Err(
                "Períodos sobrepostos: resolva a concomitância no editor antes da conferência online."
                    .into(),
            );
        }
    }
    Ok(())
}
fn json_number(value: Decimal) -> Result<Value, String> {
    let text = format!("{value:.2}");
    let number = serde_json::Number::from_str(&text)
        .map_err(|_| "Salário não pôde ser convertido para número JSON.".to_string())?;
    Ok(Value::Number(number))
}

/// Builds the provider body from the whitelist. No name, CPF, NIT, CNIS or
/// free-form note is ever copied.
pub fn build_body(input: &FabricaInput) -> Result<Value, String> {
    validate(input)?;
    let mut periodos = Vec::with_capacity(input.periodos.len());
    for (i, periodo) in input.periodos.iter().enumerate() {
        let salarios = periodo
            .salarios
            .iter()
            .map(|salario| {
                let valor = money(&salario.valor)?;
                Ok(json!({
                    "competencia": salario.competencia.trim(),
                    "valor": json_number(valor)?,
                }))
            })
            .collect::<Result<Vec<_>, String>>()?;
        periodos.push(json!({
            "dataInicial": periodo.data_inicial.trim(),
            "dataFinal": periodo.data_final.trim(),
            "descricao": format!("Período {}", i + 1),
            "contarCarencia": true,
            "prioritario": true,
            "salarios": salarios,
        }));
    }
    let mut opcoes = Map::new();
    opcoes.insert("especie".into(), json!(input.especie));
    if matches!(input.especie, 31 | 32) && input.isencao_carencia {
        opcoes.insert("isencaoCarenciaB31ouB32".into(), json!(true));
    }
    let tipo = input.tipo.as_deref().unwrap_or("COMUM");
    let mut body = json!({
        "dataApuracao": input.data_apuracao.trim(),
        "der": input.data_apuracao.trim(),
        "dadosSegurado": {
            "nascimento": input.data_nascimento.trim(),
            "sexo": input.sexo,
            "tipo": tipo,
        },
        "opcoesContagem": Value::Object(opcoes),
        "periodosContribuicao": periodos,
    });
    if matches!(input.especie, 31 | 32) {
        body["fatoGerador"] = json!(input
            .fato_gerador
            .as_deref()
            .map(str::trim)
            .unwrap_or_else(|| input.data_apuracao.trim()));
    }
    Ok(body)
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

/// Parses the provider envelope into a compact summary. The huge PBC and the
/// raw response stay out of the workspace; provenance is added by the caller.
pub fn parse_summary(bytes: &[u8]) -> Result<Value, String> {
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
    Ok(json!({
        "maiorRmi": maior_rmi,
        "moeda": first.get("moeda").cloned().unwrap_or(Value::Null),
        "beneficios": beneficios,
        "conclusao": first.get("conclusao").cloned().unwrap_or(Value::Null),
        "linhasDescartes": linhas,
        "resultados": results.len(),
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
                    },
                    FabricaSalario {
                        competencia: "1994-08".into(),
                        valor: "2000.00".into(),
                    },
                ],
            }],
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
        species.especie = 46;
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

        let mut overlap = input();
        overlap.periodos.push(FabricaPeriodo {
            data_inicial: "2019-01-01".into(),
            data_final: "2019-12-31".into(),
            salarios: vec![FabricaSalario {
                competencia: "2019-12".into(),
                valor: "2000.00".into(),
            }],
        });
        assert!(validate(&overlap).is_err());
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




