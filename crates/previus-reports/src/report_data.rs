//! Constrói o JSON de dados que cada template Typst consome, já formatado em
//! padrão brasileiro. Nunca inventa valor ausente: campo desconhecido fica vazio.
use crate::format::{br_date, brl_decimal, decimal, period_from_days_360, period_from_months};
use previus_workspace::Calculation;
use rust_decimal::Decimal;
use serde_json::{json, Value};

fn species_label(code: &str) -> String {
    match code.trim() {
        "42" => "42 - Aposentadoria Por Tempo de Contribuição ou Programada".into(),
        "32" => "32 - Invalidez ou Incapacidade Permanente Previdenciária".into(),
        "31" => "31 - Auxílio Doença Previdenciário ou Incapacidade Temporária".into(),
        other if other.is_empty() => String::new(),
        other => other.to_string(),
    }
}

fn txt(case: &Calculation, value: Option<&str>) -> String {
    let _ = case;
    value.unwrap_or("").trim().to_string()
}

fn identification(case: &Calculation, data_calculo: &str) -> Value {
    let nome = txt(case, case.identity.full_name.as_deref());
    let cpf = txt(case, case.identity.cpf.as_deref());
    let nasc = case
        .identity
        .birth_date
        .as_deref()
        .map(br_date)
        .unwrap_or_default();
    let sexo = txt(case, case.sex.as_deref());
    let especie = species_label(case.especie.as_deref().unwrap_or(""));
    json!({
        "rows": [
            [["Nome do Segurado", nome], ["Nº de Identificação do Trabalhador (NIT)", ""]],
            [["CPF", cpf], ["Data de Nascimento", nasc], ["Sexo", sexo], ["Data do Cálculo", data_calculo]],
            [["Espécie do Benefício", especie]],
            [["Regra de Cálculo", "Regra do Cálculo"]],
        ]
    })
}

fn native(outcome: Option<&Value>) -> Option<&Value> {    outcome
        .filter(|v| v.get("status").and_then(Value::as_str) == Some("experimental"))
        .and_then(|v| v.pointer("/data/native_result"))
        .filter(|v| v.is_object())
}

/// Primeiro ponteiro existente entre vários caminhos (PcD e RGPS divergem).
fn pick<'a>(n: &'a Value, paths: &[&str]) -> Option<&'a Value> {
    paths.iter().find_map(|p| n.pointer(p))
}

fn memoria_valor(n: &Value, tipo: &str) -> Option<Decimal> {
    n.pointer("/memoria")
        .and_then(Value::as_array)
        .and_then(|arr| {
            arr.iter()
                .find(|item| item.get("tipo").and_then(Value::as_str) == Some(tipo))
        })
        .and_then(|item| item.get("valor"))
        .and_then(decimal)
}

fn analysis(case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let n = native(outcome);

    let rmi = n.and_then(|v| {
        pick(v, &["/resumo/rmi", "/transicoes/rmi_escolhida"])
            .and_then(decimal)
            .or_else(|| memoria_valor(v, "rmi"))
    });
    let salario_beneficio = n.and_then(|v| {
        pick(v, &["/resumo/salario_beneficio"]).and_then(decimal)
    });
    let soma = n.and_then(|v| pick(v, &["/correcao/soma_considerada"]).and_then(decimal));
    let tempo_dias = n.and_then(|v| {
        pick(v, &["/tempo/total/days_30_360"]).and_then(Value::as_i64)
    });
    let tempo_txt = n
        .and_then(|v| {
            if let Some(t) = pick(v, &["/tempo/total"]) {
                let a = t.get("anos").and_then(Value::as_i64).unwrap_or(0);
                let m = t.get("meses").and_then(Value::as_i64).unwrap_or(0);
                let d = t.get("dias").and_then(Value::as_i64).unwrap_or(0);
                Some(crate::format::period(a, m, d))
            } else {
                tempo_dias.map(period_from_days_360)
            }
        })
        .unwrap_or_default();
    let idade_meses = n.and_then(|v| {
        pick(v, &["/pcd/idade_der_meses", "/transicoes/idade_meses"]).and_then(Value::as_i64)
    });
    let idade_txt = idade_meses.map(period_from_months).unwrap_or_default();
    let competencias = n
        .and_then(|v| {
            pick(v, &["/resumo/competencias", "/carencia/competencias_salario", "/parametros/divisor"])
                .and_then(Value::as_u64)
        })
        .unwrap_or(0);
    let sb_fallback = salario_beneficio.or_else(|| n.and_then(|v| memoria_valor(v, "media")));

    let valor = rmi.map(brl_decimal).unwrap_or_else(|| "Não apurado".into());
    let sb = sb_fallback.map(brl_decimal).unwrap_or_else(|| "Não apurado".into());
    let soma_txt = soma.map(brl_decimal).unwrap_or_else(|| "Não apurado".into());

    let requirement_tempo = json!({
        "label": "TEMPO DE CONTRIBUIÇÃO",
        "value": tempo_txt,
        "status": if rmi.is_some() { "Requisito atingido" } else { "Requisito não atingido" },
        "ok": rmi.is_some(),
    });
    let requirement_idade = json!({
        "label": "IDADE",
        "value": idade_txt,
        "status": "Sem idade mínima necessária",
        "ok": false,
    });

    Ok(json!({
        "title": "Análise da Situação Atual do Benefício",
        "date": data_calculo,
        "identification": identification(case, &data_calculo),
        "highlight": [
            ["Data de aposentadoria", data_calculo],
            ["Valor estimado da aposentadoria", valor],
        ],
        "requirements": [requirement_tempo, requirement_idade],
        "a_pagar": [
            ["Salário de contribuição", sb],
            ["Alíquota", "20%"],
            ["Contribuição mensal (INSS)", "R$ 0,00"],
            ["IRRF", "R$ 0,00"],
            ["Nº de contribuições", competencias.to_string()],
            ["Total de contribuições (INSS + IRRF)", soma_txt],
        ],
        "a_receber": [
            ["Valor estimado da aposentadoria", valor],
            ["Expectativa de sobrevida", ""],
            ["Nº estimado de meses a receber", ""],
            ["IRRF", "R$ 0,00"],
            ["Tempo para repor o investimento", ""],
            ["Total de benefício a receber", ""],
        ],
        "signature": "",
    }))
}

fn titlecase(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn periods(case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let n = native(outcome);
    let vinculos: Vec<&Value> = n
        .and_then(|v| v.pointer("/tempo/vinculos"))
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default();

    let info_headers = vec!["Início", "Fim", "Tipo", "Empresa", "Cargo"];
    let cons_headers = vec![
        "Início", "Fim", "Tipo", "Empresa", "Tempo de Contribuição", "Fator", "Total (TC x Fator)",
    ];

    let informados_rows: Vec<Vec<String>> = vinculos
        .iter()
        .map(|v| {
            vec![
                br_date(v.get("inicio").and_then(Value::as_str).unwrap_or("")),
                br_date(v.get("fim").and_then(Value::as_str).unwrap_or("")),
                titlecase(v.get("tipo").and_then(Value::as_str).unwrap_or("")),
                v.get("empresa").and_then(Value::as_str).unwrap_or("").to_string(),
                String::new(),
            ]
        })
        .collect();

    let considerados_rows: Vec<Vec<String>> = vinculos
        .iter()
        .map(|v| {
            let dias = v.get("dias").and_then(Value::as_i64).unwrap_or(0);
            let fator = v.get("fator").and_then(Value::as_f64).unwrap_or(1.0);
            let total = (dias as f64 * fator).round() as i64;
            vec![
                br_date(v.get("inicio").and_then(Value::as_str).unwrap_or("")),
                br_date(v.get("fim").and_then(Value::as_str).unwrap_or("")),
                titlecase(v.get("tipo").and_then(Value::as_str).unwrap_or("")),
                v.get("empresa").and_then(Value::as_str).unwrap_or("").to_string(),
                period_from_days_360(dias),
                format!("{fator:.2}").replace('.', ","),
                period_from_days_360(total),
            ]
        })
        .collect();

    let total_txt = n
        .and_then(|v| v.pointer("/tempo/total"))
        .map(|t| {
            crate::format::period(
                t.get("anos").and_then(Value::as_i64).unwrap_or(0),
                t.get("meses").and_then(Value::as_i64).unwrap_or(0),
                t.get("dias").and_then(Value::as_i64).unwrap_or(0),
            )
        })
        .unwrap_or_default();
    let carencia = n
        .and_then(|v| v.pointer("/carencia/carencia_meses"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let idade_txt = n
        .and_then(|v| v.pointer("/transicoes/idade_meses"))
        .and_then(Value::as_i64)
        .map(period_from_months)
        .unwrap_or_default();

    Ok(json!({
        "title": "Relatório do Tempo de Contribuição",
        "date": data_calculo,
        "identification": identification(case, &data_calculo),
        "informados": { "headers": info_headers, "rows": informados_rows },
        "legenda_informados": "Legenda: * (Contribuições consideradas na carência)",
        "considerados": { "headers": cons_headers, "rows": considerados_rows },
        "legenda_considerados": "Legenda: # (Atividades Concomitantes) / Def (Deficiência)",
        "totais": [
            ["Totais", total_txt],
            ["Carência", format!("{carencia} contribuições")],
            ["Idade", idade_txt],
        ],
        "analise": [
            ["Nº de Contribuições", carencia.to_string()],
            ["Tempo de Contribuição", total_txt],
            ["Idade", idade_txt],
        ],
    }))
}

fn transitions(case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let n = native(outcome);
    let regras: Vec<Value> = n
        .and_then(|v| v.pointer("/transicoes/regras"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|r| {
                    let nome = r.get("nome").and_then(Value::as_str).unwrap_or("").to_string();
                    let rmi = r
                        .get("rmi_final")
                        .or_else(|| r.get("rmi"))
                        .and_then(decimal)
                        .map(brl_decimal)
                        .unwrap_or_else(|| "Não apurada".into());
                    let elegivel = r.get("elegivel").and_then(Value::as_bool).unwrap_or(false);
                    let tempo = n
                        .and_then(|v| v.pointer("/tempo/total"))
                        .map(|t| {
                            crate::format::period(
                                t.get("anos").and_then(Value::as_i64).unwrap_or(0),
                                t.get("meses").and_then(Value::as_i64).unwrap_or(0),
                                t.get("dias").and_then(Value::as_i64).unwrap_or(0),
                            )
                        })
                        .unwrap_or_default();
                    let carencia = n
                        .and_then(|v| v.pointer("/carencia/carencia_meses"))
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let idade = n
                        .and_then(|v| v.pointer("/transicoes/idade_meses"))
                        .and_then(Value::as_i64)
                        .map(period_from_months)
                        .unwrap_or_default();
                    json!({
                        "nome": nome,
                        "tempo": tempo,
                        "carencia": format!("{carencia} contribuições"),
                        "idade": idade,
                        "rmi": rmi,
                        "der": data_calculo,
                        "chosen": elegivel,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(json!({
        "title": "Análise das Regras de Transição - EC 103/2019",
        "date": data_calculo,
        "identification": identification(case, &data_calculo),
        "regras": regras,
        "nota": "As projeções futuras são feitas com a repetição do último salário de contribuição a partir da DIB informada, gerando RMI's com valores aproximados.",
    }))
}

/// Constrói os dados de um relatório a partir do caso e do resultado nativo.
pub fn build(kind: &str, case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    match kind {
        "analysis" => analysis(case, outcome),
        "periods" => periods(case, outcome),
        "transitions" => transitions(case, outcome),
        other => Err(format!("Relatório sem construtor de dados: {other}")),
    }
}
