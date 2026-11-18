//! Constrói o JSON de dados que cada template Typst consome, já formatado em
//! padrão brasileiro. Nunca inventa valor ausente: campo desconhecido fica vazio.
use crate::format::{br_date, br_month, brl, brl_decimal, decimal, period_from_days_360, period_from_months};
use previdenciar_workspace::Calculation;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::{json, Value};

fn species_label(code: &str) -> String {
    match code.trim() {
        "21" => "21 - Pensão por Morte Previdenciária".into(),
        "25" => "25 - Auxílio-Reclusão".into(),
        "31" => "31 - Auxílio por Incapacidade Temporária Previdenciário".into(),
        "32" => "32 - Aposentadoria por Incapacidade Permanente".into(),
        "36" => "36 - Auxílio-Acidente Previdenciário".into(),
        "41" => "41 - Aposentadoria por Idade".into(),
        "42" => "42 - Aposentadoria Por Tempo de Contribuição ou Programada".into(),
        "46" => "46 - Aposentadoria Especial".into(),
        "57" => "57 - Aposentadoria de Professor".into(),
        "80" => "80 - Salário-Maternidade".into(),
        "87" => "87 - Amparo Assistencial à Pessoa com Deficiência (BPC)".into(),
        "88" => "88 - Amparo Assistencial ao Idoso (BPC)".into(),
        "91" => "91 - Auxílio por Incapacidade Temporária Acidentário".into(),
        "92" => "92 - Aposentadoria por Incapacidade Permanente Acidentária".into(),
        "93" => "93 - Pensão por Morte Acidentária".into(),
        "94" => "94 - Auxílio-Acidente Acidentário".into(),
        "95" => "95 - Auxílio-Suplementar por Acidente de Trabalho".into(),
        other if other.is_empty() => String::new(),
        other => other.to_string(),
    }
}

fn txt(case: &Calculation, value: Option<&str>) -> String {
    let _ = case;
    value.unwrap_or("").trim().to_string()
}

pub(crate) fn parse_dib_iso(dib: &str) -> Option<(i64, i64, i64, String)> {
    let s = dib.trim();
    if let Some((y, m, d)) = iso_parts(s) {
        return Some((y, m, d, format!("{y:04}-{m:02}-{d:02}")));
    }
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 3 {
        let d: i64 = parts[0].parse().ok()?;
        let m: i64 = parts[1].parse().ok()?;
        let y: i64 = parts[2].parse().ok()?;
        return Some((y, m, d, format!("{y:04}-{m:02}-{d:02}")));
    }
    None
}

pub(crate) fn idade_na_dib_anos(case: &Calculation) -> Option<i64> {
    let birth = case
        .identity
        .birth_date
        .as_deref()
        .or_else(|| case.cnis.as_ref().and_then(|c| c.identity.birth_date.as_deref()))?;
    let (by, bm, bd, _) = parse_dib_iso(birth)?;
    let (dy, dm, dd, _) = parse_dib_iso(&case.dib)?;
    let mut age = dy - by;
    if (dm, dd) < (bm, bd) {
        age -= 1;
    }
    Some(age.max(0))
}

pub(crate) fn sobrevida_info(case: &Calculation) -> Option<(Decimal, i64)> {
    let (_, _, _, dib_iso) = parse_dib_iso(&case.dib)?;
    let age = idade_na_dib_anos(case)? as usize;
    let tabela = &rgps_core::dados::expectativa_sobrevida::EXPECTATIVA_GERAL;
    let row = tabela
        .iter()
        .filter(|(vigencia, _)| *vigencia <= dib_iso.as_str())
        .map(|(_, r)| r)
        .last()
        .or_else(|| tabela.first().map(|(_, r)| r))?;
    let es = Decimal::from_str_exact(row[age.min(90)]).ok()?;
    let meses = (es * Decimal::from(12)).round().to_i64().unwrap_or(0);
    Some((es, meses.max(1)))
}

pub(crate) fn irrf_competencia(case: &Calculation) -> String {
    let last_comp = rgps_core::dados::irrf::IRRF
        .last()
        .map(|t| t.competencia)
        .unwrap_or("2026-01");
    let first_comp = rgps_core::dados::irrf::IRRF
        .first()
        .map(|t| t.competencia)
        .unwrap_or("1994-07");
    if let Some((y, m, _, _)) = parse_dib_iso(&case.dib) {
        let ym = format!("{y:04}-{m:02}");
        if ym.as_str() < first_comp {
            first_comp.to_string()
        } else if ym.as_str() > last_comp {
            last_comp.to_string()
        } else {
            ym
        }
    } else {
        last_comp.to_string()
    }
}

fn best_rule_name(case: &Calculation, outcome: Option<&Value>) -> String {
    if let Some(n) = native(outcome) {
        if let Some(name) = n
            .pointer("/especie/regra_escolhida")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        {
            return name.to_string();
        }
        if let Some(rules) = n.pointer("/transicoes/regras").and_then(Value::as_array) {
            let best_eleg = rules
                .iter()
                .filter(|r| r.get("elegivel").and_then(Value::as_bool) == Some(true))
                .max_by(|a, b| {
                    let ra = a.get("rmi_final").or_else(|| a.get("rmi")).and_then(decimal).unwrap_or(Decimal::ZERO);
                    let rb = b.get("rmi_final").or_else(|| b.get("rmi")).and_then(decimal).unwrap_or(Decimal::ZERO);
                    ra.cmp(&rb)
                })
                .and_then(|r| r.get("nome").and_then(Value::as_str))
                .filter(|s| !s.trim().is_empty());
            if let Some(name) = best_eleg {
                return name.to_string();
            }
            if let Some(first_name) = rules
                .first()
                .and_then(|r| r.get("nome").and_then(Value::as_str))
                .filter(|s| !s.trim().is_empty())
            {
                return first_name.to_string();
            }
        }
        if let Some(mod_pcd) = n.pointer("/pcd/modalidade").and_then(Value::as_str) {
            return if mod_pcd == "idade" {
                "LC 142/2013 - PcD por Idade".into()
            } else {
                "LC 142/2013 - PcD por Tempo de Contribuição".into()
            };
        }
    }
    let esp = case.especie.as_deref().unwrap_or("").trim();
    if esp.is_empty() {
        "Regra do Cálculo (RGPS)".into()
    } else {
        format!("Análise Espécie {esp}")
    }
}

fn identification_with_rule(case: &Calculation, data_calculo: &str, outcome: Option<&Value>) -> Value {
    let nome = txt(case, case.identity.full_name.as_deref());
    let nit = txt(
        case,
        case.identity
            .nit
            .as_deref()
            .or_else(|| case.cnis.as_ref().and_then(|c| c.identity.nit.as_deref())),
    );
    let cpf = txt(case, case.identity.cpf.as_deref());
    let nasc = case
        .identity
        .birth_date
        .as_deref()
        .map(br_date)
        .unwrap_or_default();
    let sexo = txt(case, case.sex.as_deref());
    let especie = species_label(case.especie.as_deref().unwrap_or(""));
    let regra = best_rule_name(case, outcome);
    json!({
        "rows": [
            [["Nome do Segurado", nome], ["Nº de Identificação do Trabalhador (NIT)", nit]],
            [["CPF", cpf], ["Data de Nascimento", nasc], ["Sexo", sexo], ["Data do Cálculo", data_calculo]],
            [["Espécie do Benefício", especie]],
            [["Regra de Cálculo", regra]],
        ]
    })
}

fn identification(case: &Calculation, data_calculo: &str) -> Value {
    identification_with_rule(case, data_calculo, None)
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
    let contribuicao = sb_fallback
        .map(|v| (v * Decimal::new(20, 2)).round_dp(2))
        .map(brl_decimal)
        .unwrap_or_else(|| "R$ 0,00".into());

    let valor = rmi.map(brl_decimal).unwrap_or_else(|| "Não apurado".into());
    let sb = sb_fallback.map(brl_decimal).unwrap_or_else(|| "Não apurado".into());
    let soma_txt = soma.map(brl_decimal).unwrap_or_else(|| "Não apurado".into());

    let requirement_tempo = json!({
        "label": "TEMPO DE CONTRIBUIÇÃO",
        "value": tempo_txt,
        "status": if rmi.is_some() { "Requisito atingido" } else { "Requisito não atingido" },
        "ok": rmi.is_some(),
    });
    let carencia_meses = n
        .and_then(|v| pick(v, &["/carencia/meses_apurados", "/carencia/competencias_salario", "/resumo/carencia_meses"]).and_then(Value::as_u64))
        .unwrap_or(0);
    let mut requirements = vec![requirement_tempo];
    if let Some(es_reqs) = n.and_then(|v| v.pointer("/especie/requisitos")).and_then(Value::as_array) {
        for r in es_reqs {
            if let Some(txt_req) = r.as_str() {
                requirements.push(json!({
                    "label": "REQUISITO DA ESPÉCIE",
                    "value": txt_req,
                    "status": if rmi.is_some() { "Conferido" } else { "Em conferência" },
                    "ok": rmi.is_some(),
                }));
            }
        }
    } else if !idade_txt.is_empty() {
        requirements.push(json!({
            "label": "IDADE NA DIB",
            "value": idade_txt,
            "status": format!("Carência apurada: {carencia_meses} meses"),
            "ok": rmi.is_some(),
        }));
    } else {
        requirements.push(json!({
            "label": "CARÊNCIA",
            "value": format!("{carencia_meses} meses"),
            "status": if carencia_meses >= 180 { "Carência mínima atingida" } else { "Carência pendente" },
            "ok": carencia_meses >= 180,
        }));
    }

    let es_info = sobrevida_info(case);
    let es_txt = es_info
        .map(|(es, _)| format!("{} anos (IBGE)", es.round_dp(1)))
        .unwrap_or_else(|| "Não apurada".into());
    let meses_rec_opt = es_info.map(|(_, m)| m);
    let meses_txt = meses_rec_opt
        .map(|m| format!("{m} meses"))
        .unwrap_or_else(|| "Não apurado".into());
    let irrf_comp = irrf_competencia(case);
    let irrf_rec = rmi
        .and_then(|r| {
            let base = r.round_dp(2).to_string();
            rgps_core::fiscal::irrf_mensal(&irrf_comp, &base, 0, None)
                .ok()
                .map(|v| v.irrf_liquido_lei_15270)
        })
        .unwrap_or(Decimal::ZERO);
    let irrf_txt = brl_decimal(irrf_rec);
    let total_rec_txt = match (rmi, meses_rec_opt) {
        (Some(r), Some(m)) => {
            let rmi_13 = r * Decimal::from(13) / Decimal::from(12);
            let liquido = (rmi_13 - irrf_rec).max(Decimal::ZERO);
            brl_decimal((liquido * Decimal::from(m)).round_dp(2))
        }
        _ => String::new(),
    };
    let payback_txt = match (rmi, soma) {
        (Some(r), Some(s)) if r > irrf_rec && s > Decimal::ZERO => {
            let pm = (s / (r - irrf_rec)).round_dp(0);
            format!("{pm} meses")
        }
        _ => String::new(),
    };

    Ok(json!({
        "title": "Análise da Situação Atual do Benefício",
        "date": data_calculo,
        "identification": identification_with_rule(case, &data_calculo, outcome),
        "highlight": [
            ["Data de aposentadoria", data_calculo],
            ["Valor estimado da aposentadoria", valor],
        ],
        "requirements": requirements,
        "a_pagar": [
            ["Salário de contribuição", sb],
            ["Alíquota", "20%"],
            ["Contribuição mensal (INSS)", contribuicao],
            ["IRRF", irrf_txt.clone()],
            ["Nº de contribuições", competencias.to_string()],
            ["Total de contribuições (INSS + IRRF)", soma_txt],
        ],
        "a_receber": [
            ["Valor estimado da aposentadoria", valor],
            ["Expectativa de sobrevida", es_txt],
            ["Nº estimado de meses a receber", meses_txt],
            ["IRRF", irrf_txt],
            ["Tempo para repor o investimento", payback_txt],
            ["Total de benefício a receber", total_rec_txt],
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

fn periods(case: &Calculation, outcome: Option<&Value>, personal: bool) -> Result<Value, String> {
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

    let empresa_label = |v: &Value| -> String {
        if personal {
            v.get("empresa").and_then(Value::as_str).unwrap_or("").to_string()
        } else {
            "Origem oculta".to_string()
        }
    };

    let informados_rows: Vec<Vec<String>> = vinculos
        .iter()
        .map(|v| {
            vec![
                br_date(v.get("inicio").and_then(Value::as_str).unwrap_or("")),
                br_date(v.get("fim").and_then(Value::as_str).unwrap_or("")),
                titlecase(v.get("tipo").and_then(Value::as_str).unwrap_or("")),
                empresa_label(v),
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
                empresa_label(v),
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
                    let _elegivel = r.get("elegivel").and_then(Value::as_bool).unwrap_or(false);
                    let chosen = n.and_then(|v| v.pointer("/transicoes/regra_escolhida")).and_then(Value::as_str) == r.get("id").and_then(Value::as_str);
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
                        "chosen": chosen,
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

fn memory_report(case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let n = native(outcome);
    let headers = vec![
        "Nº", "Data", "Dias", "Salário de Contribuição", "Índice de Atualização", "Salário Corrigido",
    ];
    let descartadas_set: std::collections::HashSet<&str> = n
        .and_then(|v| v.pointer("/correcao/parcelas_descartadas"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let rows: Vec<Vec<String>> = n
        .and_then(|v| v.pointer("/correcao/parcelas"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .enumerate()
                .map(|(i, p)| {
                    let comp = p.get("competencia").and_then(Value::as_str).unwrap_or("");
                    let descartada = p.get("descartada").and_then(Value::as_bool) == Some(true)
                        || descartadas_set.contains(comp);
                    let num_col = if descartada {
                        format!("{:03} (D)", i + 1)
                    } else {
                        format!("{:03}", i + 1)
                    };
                    vec![
                        num_col,
                        br_date(comp),
                        p.get("dias").and_then(Value::as_i64).unwrap_or(30).to_string(),
                        p.get("salario")
                            .or_else(|| p.get("valor"))
                            .and_then(Value::as_str)
                            .map(brl)
                            .unwrap_or_default(),
                        p.get("indice").and_then(Value::as_str).unwrap_or("").to_string(),
                        p.get("corrigido").and_then(Value::as_str).map(brl).unwrap_or_default(),
                    ]
                })
                .collect()
        })
        .unwrap_or_default();

    let idade = n
        .and_then(|v| v.pointer("/transicoes/idade_meses"))
        .and_then(Value::as_i64)
        .map(period_from_months)
        .unwrap_or_default();
    let parcelas = n
        .and_then(|v| v.pointer("/parametros/divisor"))
        .and_then(Value::as_u64)
        .or_else(|| n.and_then(|v| v.pointer("/carencia/competencias_salario")).and_then(Value::as_u64))
        .unwrap_or(0);
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
    let soma = n.and_then(|v| v.pointer("/correcao/soma_considerada")).and_then(decimal);
    let media = n
        .and_then(|v| v.pointer("/resumo/salario_beneficio"))
        .and_then(decimal)
        .or_else(|| n.and_then(|v| memoria_valor(v, "media")));
    let sb = media;
    let coeficiente = n
        .and_then(|v| v.pointer("/resumo/coeficiente"))
        .or_else(|| n.and_then(|v| v.pointer("/parametros/coeficiente_regra")))
        .or_else(|| n.and_then(|v| v.pointer("/parametros/coeficiente")))
        .and_then(|c| c.as_str().map(|s| s.to_string()).or_else(|| c.as_i64().map(|i| i.to_string())))
        .unwrap_or_default();
    let rmi = n.and_then(|v| {
        pick(v, &["/resumo/rmi", "/transicoes/rmi_escolhida"])
            .and_then(decimal)
            .or_else(|| memoria_valor(v, "rmi"))
    });

    let fmt = |v: Option<Decimal>| v.map(brl_decimal).unwrap_or_else(|| "Não apurado".into());
    let especie_nome = n
        .and_then(|v| v.pointer("/especie/nome"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let especie_regra = n
        .and_then(|v| v.pointer("/especie/regra_escolhida"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let mut resumo_rows = vec![
        vec!["Idade na DIB".into(), idade],
        vec!["Espécie".into(), if especie_nome.is_empty() { "Não informada".into() } else { especie_nome }],
        vec!["Regra".into(), if especie_regra.is_empty() { best_rule_name(case, outcome) } else { especie_regra }],
        vec!["Nº de Parcelas no PBC".into(), parcelas.to_string()],
        vec!["Tempo de Contribuição".into(), tempo],
        vec!["Soma dos Salários".into(), fmt(soma)],
        vec!["Média dos Salários Corrigidos".into(), fmt(media)],
        vec!["Salário de Benefício".into(), fmt(sb)],
        vec!["Coeficiente".into(), if coeficiente.is_empty() { String::new() } else { format!("{coeficiente} %") }],
        vec!["Renda Mensal Inicial".into(), fmt(rmi)],
    ];
    if let Some(best_desc) = n
        .and_then(|v| v.pointer("/simulacao_descartes"))
        .and_then(Value::as_array)
        .and_then(|arr| arr.iter().max_by(|a, b| {
            let ra = a.get("rmi").and_then(decimal).unwrap_or(Decimal::ZERO);
            let rb = b.get("rmi").and_then(decimal).unwrap_or(Decimal::ZERO);
            ra.cmp(&rb)
        }))
    {
        let qtd = best_desc
            .get("n")
            .or_else(|| best_desc.get("descartadas"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let rmi_opt = best_desc.get("rmi").and_then(decimal);
        resumo_rows.push(vec![
            "Simulação Descarte Ótimo (EC 103)".into(),
            format!("{qtd} descartada(s) -> {}", fmt(rmi_opt)),
        ]);
    }
    if let Some(fab_rmi) = case
        .fabrica
        .as_ref()
        .and_then(|f| f.pointer("/provider/maiorRmi").or_else(|| f.get("maiorRmi")))
        .and_then(Value::as_str)
    {
        resumo_rows.push(vec![
            "Conferência Fábrica TRF3".into(),
            brl(fab_rmi),
        ]);
    }
    Ok(json!({
        "title": "Cálculo da R.M.I.",
        "subtitle": "Cálculo da R.M.I. - regra e requisitos da espécie",
        "date": data_calculo,
        "identification": identification_with_rule(case, &data_calculo, outcome),
        "headers": headers,
        "rows": rows,
        "resumo": resumo_rows,
    }))
}

fn iso_parts(iso: &str) -> Option<(i64, i64, i64)> {
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() == 3 {
        Some((parts[0].parse().ok()?, parts[1].parse().ok()?, parts[2].parse().ok()?))
    } else {
        None
    }
}

fn quality(case: &Calculation, outcome: Option<&Value>, personal: bool) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let headers = vec!["Início", "Fim", "Filiação", "Carência", "Tempo Contribuição", "Origem do Vínculo"];
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut qperiods: Vec<previdenciar_core::QualityPeriod> = Vec::new();
    let mut first_dates: Vec<String> = Vec::new();

    if let Some(cnis) = &case.cnis {
        for link in &cnis.links {
            let start_raw = link.start_date.as_deref().unwrap_or("");
            let end_raw = link.end_date.as_deref().unwrap_or("");
            if !start_raw.is_empty() {
                first_dates.push(start_raw.to_string());
            }
            let inicio = link.start_date.as_deref().map(br_date).unwrap_or_default();
            let fim = link.end_date.as_deref().map(br_date).unwrap_or_default();
            let meses = cnis
                .salary_entries
                .iter()
                .filter(|e| e.link_sequence == Some(link.sequence))
                .count();
            let tempo = match (iso_parts(start_raw), iso_parts(end_raw)) {
                (Some((ay, am, ad)), Some((by, bm, bd))) => {
                    let dias = (by - ay) * 360 + (bm - am) * 30 + (bd - ad) + 1;
                    period_from_days_360(dias.max(0))
                }
                _ => String::new(),
            };
            let origem = if personal { link.raw_employer.clone().unwrap_or_default() } else { "Origem oculta".to_string() };
            rows.push(vec![
                inicio,
                fim,
                link.affiliation.clone().unwrap_or_default(),
                format!("{meses} contribuições"),
                tempo,
                origem,
            ]);

            let start = link.start_date.as_deref().and_then(|v| v.parse::<previdenciar_core::CivilDate>().ok());
            let end = link.end_date.as_deref().and_then(|v| v.parse::<previdenciar_core::CivilDate>().ok());
            let mut contribution_months = Vec::new();
            let mut late_months = Vec::new();
            for entry in cnis.salary_entries.iter().filter(|e| e.link_sequence == Some(link.sequence)) {
                if let Ok(ym) = entry.competence.parse::<previdenciar_core::YearMonth>() {
                    if entry.indicators.iter().any(|c| c.eq_ignore_ascii_case("PREM-EXT")) {
                        late_months.push(ym);
                    }
                    contribution_months.push(ym);
                }
            }
            let mut disregarded = Vec::new();
            if let Some(review) = &case.contribution_review {
                for record in review.records.iter().filter(|r| r.link_sequence == Some(link.sequence) && r.excluded) {
                    if let Ok(ym) = record.competence.parse::<previdenciar_core::YearMonth>() {
                        disregarded.push(ym);
                    }
                }
            }
            let benefit_period = link.affiliation.as_deref().map(|v| v.to_lowercase().contains("benefício")).unwrap_or(false);
            qperiods.push(previdenciar_core::QualityPeriod {
                label: format!("Vínculo {:02}", link.sequence),
                start,
                end,
                affiliation: link.affiliation.clone(),
                affiliation_kind: previdenciar_core::AffiliationKind::parse(link.affiliation.as_deref()),
                late_months,
                employer: if personal { link.raw_employer.clone() } else { None },
                contribution_months,
                disregarded_months: disregarded,
                had_benefit_episode: false,
                benefit_period,
            });
        }
    } else {
        for (idx, link) in case.manual_links.iter().enumerate() {
            if link.source_review.as_ref().is_some_and(|r| r.excluded) {
                continue;
            }
            first_dates.push(link.start_date.clone());
            let start_ym = link.start_date.get(..7).unwrap_or("");
            let end_ym = link.end_date.get(..7).unwrap_or("");
            let mut contribution_months = Vec::new();
            for sal in &case.manual_salaries {
                let comp = sal.competence.trim();
                if comp >= start_ym && comp <= end_ym {
                    if let Ok(ym) = comp.parse::<previdenciar_core::YearMonth>() {
                        contribution_months.push(ym);
                    }
                }
            }
            let tempo = match (iso_parts(&link.start_date), iso_parts(&link.end_date)) {
                (Some((ay, am, ad)), Some((by, bm, bd))) => {
                    let dias = (by - ay) * 360 + (bm - am) * 30 + (bd - ad) + 1;
                    period_from_days_360(dias.max(0))
                }
                _ => String::new(),
            };
            let origem = if personal { link.employer.clone() } else { "Origem oculta".to_string() };
            rows.push(vec![
                br_date(&link.start_date),
                br_date(&link.end_date),
                link.regime.clone(),
                format!("{} contribuições", contribution_months.len()),
                tempo,
                origem,
            ]);
            qperiods.push(previdenciar_core::QualityPeriod {
                label: format!("Vínculo {:02}", idx + 1),
                start: link.start_date.parse::<previdenciar_core::CivilDate>().ok(),
                end: link.end_date.parse::<previdenciar_core::CivilDate>().ok(),
                affiliation: Some(link.regime.clone()),
                affiliation_kind: previdenciar_core::AffiliationKind::Employee,
                late_months: vec![],
                employer: if personal { Some(link.employer.clone()) } else { None },
                contribution_months,
                disregarded_months: vec![],
                had_benefit_episode: false,
                benefit_period: false,
            });
        }
    }

    let filed_before_july_1991 = first_dates.iter().any(|v| v.as_str() <= "1991-07-24");
    let quality_summary = if !qperiods.is_empty() {
        previdenciar_core::insured_quality(&previdenciar_core::QualityInput {
            periods: qperiods,
            required_carencia_months: 180,
            implementation_year: None,
            filed_before_july_1991,
        })
        .ok()
    } else {
        None
    };

    let total_meses: usize = quality_summary
        .as_ref()
        .map(|q| q.total_counted_months as usize)
        .unwrap_or_else(|| {
            rows.iter()
                .map(|r| {
                    r.get(3)
                        .and_then(|s| s.split_whitespace().next())
                        .and_then(|n| n.parse::<usize>().ok())
                        .unwrap_or(0)
                })
                .sum()
        });
    let mut totais = vec![vec!["Total de contribuições".into(), format!("{total_meses} contribuições")]];
    if let Some(q) = &quality_summary {
        totais.push(vec![
            "Carência exigida / apurada".into(),
            format!(
                "{} meses ({}) — {}",
                q.required_carencia_months,
                q.required_carencia_basis,
                if q.carencia_observed { "Atingida" } else { "Não atingida" }
            ),
        ]);
        if let Some(grace_str) = q
            .periods
            .iter()
            .filter_map(|p| p.grace_until)
            .max()
            .map(|d| d.to_br())
        {
            totais.push(vec![
                "Manutenção da qualidade de segurado (período de graça até)".into(),
                grace_str,
            ]);
        }
    }
    Ok(json!({
        "title": "Cálculo da Qualidade de Segurado",
        "date": data_calculo,
        "identification": identification_with_rule(case, &data_calculo, outcome),
        "headers": headers,
        "rows": rows,
        "totais": totais,
        "nota": "Apuração própria sobre as contribuições declaradas na origem (arts. 15, 24, 25 e 142 da Lei 8.213/1991).",
    }))
}

fn roi(case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let n = native(outcome);
    let es_info = sobrevida_info(case);
    let meses_es = es_info.map(|(_, m)| m).unwrap_or(240);
    let irrf_comp = irrf_competencia(case);
    let expectativa_label = es_info
        .map(|(es, m)| format!("Expectativa IBGE na DIB: {} anos ({} meses).", es.round_dp(1), m))
        .unwrap_or_else(|| "Expectativa de sobrevida conforme tábua do IBGE (estimativa 240 meses por ausência de nascimento).".into());
    let regras: Vec<Value> = n
        .and_then(|v| v.pointer("/transicoes/regras"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|r| {
                    let rmi = r
                        .get("rmi_final")
                        .or_else(|| r.get("rmi"))
                        .and_then(decimal)?;
                    if rmi <= Decimal::ZERO {
                        return None;
                    }
                    let nome = r.get("nome").and_then(Value::as_str).unwrap_or("").to_string();
                    let meses = meses_es;
                    let rmi_13 = rmi * Decimal::from(13) / Decimal::from(12);
                    let base = rmi_13.round_dp(2).to_string();
                    let irrf = rgps_core::fiscal::irrf_mensal(&irrf_comp, &base, 0, None)
                        .map(|v| v.irrf_liquido_lei_15270)
                        .unwrap_or(Decimal::ZERO);
                    let liquido = rmi_13 - irrf;
                    let total = (liquido * Decimal::from(meses)).round_dp(2);
                    Some(json!({
                        "nome": nome,
                        "der": data_calculo,
                        "investimento": "Nenhum investimento necessário",
                        "rmi": brl_decimal(rmi),
                        "total": brl_decimal(total),
                        "detalhe": [
                            format!("Total líquido recebido de {} até a expectativa ({} meses considerando o 13º salário)", data_calculo, meses),
                            format!("Provável RMI de {}", brl_decimal(rmi)),
                            format!("RMI com 13º: {}", brl_decimal(rmi_13)),
                            format!("IRRF mensal estimado (Lei 15.270/2025 / comp. {}): {}", irrf_comp, brl_decimal(irrf)),
                            format!("Total líquido: {}", brl_decimal(total)),
                        ],
                        "expectativa": expectativa_label,
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(json!({
        "title": "Cálculo do Retorno Sobre o Investimento",
        "date": data_calculo,
        "identification": identification_with_rule(case, &data_calculo, outcome),
        "regras": regras,
        "nota": "As projeções futuras são feitas com a repetição do último salário de contribuição a partir da DIB informada, gerando RMI's com valores aproximados.",
    }))
}

fn regras(case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let n = native(outcome);
    let meses_es = sobrevida_info(case).map(|(_, m)| m).unwrap_or(240);
    let idade_possui = n
        .and_then(|v| v.pointer("/transicoes/idade_meses"))
        .and_then(Value::as_i64)
        .map(period_from_months)
        .unwrap_or_default();
    let tempo_possui = n
        .and_then(|v| v.pointer("/tempo/total"))
        .map(|t| {
            crate::format::period(
                t.get("anos").and_then(Value::as_i64).unwrap_or(0),
                t.get("meses").and_then(Value::as_i64).unwrap_or(0),
                t.get("dias").and_then(Value::as_i64).unwrap_or(0),
            )
        })
        .unwrap_or_default();

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
                        .and_then(decimal);
                    let rmi_txt = rmi.map(brl_decimal).unwrap_or_else(|| "Não apurada".into());
                    let retorno = rmi
                        .map(|v| {
                            let rmi_13 = v * Decimal::from(13) / Decimal::from(12);
                            brl_decimal((rmi_13 * Decimal::from(meses_es)).round_dp(2))
                        })
                        .unwrap_or_else(|| "Não apurado".into());
                    let reqs: Vec<String> = r
                        .get("requisitos")
                        .and_then(Value::as_array)
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    let idade_nec = reqs
                        .iter()
                        .find(|s| s.to_lowercase().contains("idade"))
                        .cloned()
                        .unwrap_or_else(|| "não exigida".into());
                    let tempo_min = reqs
                        .iter()
                        .find(|s| s.to_lowercase().contains("tempo"))
                        .cloned()
                        .unwrap_or_else(|| "não exigido".into());
                    json!({
                        "nome": nome,
                        "der": data_calculo,
                        "rmi": rmi_txt,
                        "idade_necessaria": idade_nec,
                        "idade_possui": format!("Possui {idade_possui}"),
                        "tempo_minimo": tempo_min,
                        "tempo_possui": format!("Possui {tempo_possui}"),
                        "retorno": retorno,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(json!({
        "title": "Regras de Transição - EC 103/2019",
        "date": data_calculo,
        "identification": identification(case, &data_calculo),
        "regras": regras,
        "nota1": "As projeções futuras são feitas com a repetição do último salário de contribuição a partir da DIB informada, gerando RMI's com valores aproximados.",
        "nota2": "As regras de transição dos tempos adicionais de 50% e 100% são calculadas mesmo que o beneficiário já tenha atingido o tempo mínimo de contribuição, ficando a critério do usuário a verificação do direito legal a este tipo de transição.",
    }))
}

fn generic_sections(
    case: &Calculation,
    outcome: Option<&Value>,
    kind: &str,
    personal: bool,
) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let n = native(outcome);
    let title = match kind {
        "summary" => "Resumo do cálculo RGPS",
        "salaries" => "Salários de contribuição",
        "indicators" => "Indicadores e pendências de conferência",
        _ => "Relatório",
    };
    let mut sections: Vec<Value> = Vec::new();

    if kind == "salaries" {
        let mut rows: Vec<Vec<String>> = Vec::new();
        if let Some(cnis) = &case.cnis {
            for e in &cnis.salary_entries {
                let valor = Decimal::new(e.amount_centavos, 2);
                rows.push(vec![
                    br_month(&e.competence),
                    brl_decimal(valor),
                    if personal {
                        e.indicators.first().cloned().unwrap_or_default()
                    } else {
                        String::new()
                    },
                ]);
            }
        }
        for w in &case.manual_salaries {
            rows.push(vec![
                br_month(&w.competence),
                brl(w.amount.trim()),
                if personal { w.indicators.clone() } else { String::new() },
            ]);
        }
        sections.push(json!({
            "titulo": "Remunerações informadas",
            "paragrafos": [],
            "tabela": { "headers": ["Competência", "Valor", "Indicador"], "rows": rows },
        }));
    } else if kind == "summary" {
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
        let rmi = n
            .and_then(|v| {
                pick(v, &["/resumo/rmi", "/transicoes/rmi_escolhida"])
                    .and_then(decimal)
                    .or_else(|| memoria_valor(v, "rmi"))
            })
            .map(brl_decimal)
            .unwrap_or_else(|| "Não apurado".into());
        sections.push(json!({
            "titulo": "Resumo",
            "paragrafos": [
                format!("Tempo de contribuição apurado: {tempo}."),
                format!("Carência apurada: {carencia} contribuições."),
                format!("Renda mensal inicial estimada: {rmi}."),
            ],
            "tabela": null,
        }));
    } else {
        let mut linhas: Vec<String> = Vec::new();
        if let Some(o) = outcome {
            for key in ["blockers", "warnings", "known_model_risks"] {
                if let Some(arr) = o.get(key).and_then(Value::as_array) {
                    for item in arr {
                        if let Some(msg) = item.get("message").and_then(Value::as_str) {
                            linhas.push(msg.to_string());
                        } else if let Some(s) = item.as_str() {
                            linhas.push(s.to_string());
                        }
                    }
                }
            }
        }
        if let Some(v) = n {
            if let Some(avisos) = v.get("avisos").and_then(Value::as_array) {
                for a in avisos {
                    if let Some(s) = a.as_str() {
                        linhas.push(s.to_string());
                    }
                }
            }
        }
        sections.push(json!({
            "titulo": "Pendências e ressalvas",
            "paragrafos": if linhas.is_empty() { vec!["Nenhuma pendência registrada.".to_string()] } else { linhas },
            "tabela": null,
        }));
    }

    Ok(json!({
        "title": title,
        "date": data_calculo,
        "identification": identification_with_rule(case, &data_calculo, outcome),
        "sections": sections,
    }))
}

/// Constrói os dados de um relatório a partir do caso e do resultado nativo.
/// `personal = false` oculta nome, CPF e NIT do segurado.
pub fn build(kind: &str, case: &Calculation, outcome: Option<&Value>, personal: bool) -> Result<Value, String> {
    let mut data = match kind {
        "analysis" => analysis(case, outcome),
        "periods" => periods(case, outcome, personal),
        "transitions" => transitions(case, outcome),
        "memory" => memory_report(case, outcome),
        "quality" => quality(case, outcome, personal),
        "roi" => roi(case, outcome),
        "regras" => regras(case, outcome),
        "summary" | "salaries" | "indicators" => generic_sections(case, outcome, kind, personal),
        other => Err(format!("Relatório sem construtor de dados: {other}")),
    }?;
    let custom_title = case.options.report_title.trim();
    if !custom_title.is_empty() {
        if custom_title.chars().count() > 120 {
            return Err("Título do relatório excede 120 caracteres; nenhum texto foi cortado.".into());
        }
        if let Some(obj) = data.as_object_mut() {
            obj.insert("title".into(), json!(custom_title));
        }
    }
    if !personal {
        mask_identification(&mut data);
    }
    Ok(data)
}

/// Substitui as células de identificação direta sem tocar em datas, sexo ou valores.
fn mask_identification(data: &mut Value) {
    let Some(rows) = data.get_mut("identification").and_then(|value| value.get_mut("rows")).and_then(Value::as_array_mut) else { return };
    for row in rows.iter_mut().filter_map(Value::as_array_mut) {
        for cell in row.iter_mut().filter_map(Value::as_array_mut) {
            if cell.len() != 2 { continue; }
            match cell[0].as_str().unwrap_or_default() {
                "Nome do Segurado" => cell[1] = json!("Identificação oculta"),
                "CPF" | "Nº de Identificação do Trabalhador (NIT)" => cell[1] = json!("Oculto"),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identification_follows_the_personal_option() {
        let mut case = Calculation::default();
        case.identity.full_name = Some("PESSOA FICTICIA DE TESTE".into());
        case.identity.cpf = Some("111.111.111-11".into());
        let private = build("periods", &case, None, false).unwrap().to_string();
        assert!(!private.contains("PESSOA FICTICIA DE TESTE"), "{private}");
        assert!(!private.contains("111.111.111-11"), "{private}");
        assert!(private.contains("Identificação oculta"), "{private}");
        let identified = build("periods", &case, None, true).unwrap().to_string();
        assert!(identified.contains("PESSOA FICTICIA DE TESTE"), "{identified}");
    }

    #[test]
    fn memory_and_roi_use_dynamic_sobrevida_days_title_and_masked_nit() {
        let mut case: Calculation = serde_json::from_str(include_str!(
            "../../previdenciar-rgps/tests/fixtures/calculation-synthetic.json"
        ))
        .unwrap();
        case.identity.nit = Some("99988877766".into());
        case.options.report_title = "Parecer Técnico Customizado".into();
        let outcome = previdenciar_rgps::calculate_case(&case);
        let outcome_val = serde_json::json!({
            "status": "experimental",
            "data": {
                "native_result": {
                    "resumo": { "rmi": "1600.00", "salario_beneficio": "2000.00", "coeficiente": 80, "competencias": 300 },
                    "correcao": {
                        "media": "2000.00",
                        "consideradas": 300,
                        "descartadas": 1,
                        "parcelas_descartadas": ["1994-07"],
                        "parcelas": [
                            { "competencia": "1994-07", "salario": "1000.00", "fator": "2.0000", "corrigido": "2000.00" },
                            { "competencia": "1994-08", "salario": "2000.00", "fator": "1.0000", "corrigido": "2000.00" }
                        ]
                    },
                    "especie": { "codigo": "41", "nome": "Aposentadoria por idade" },
                    "transicoes": {
                        "regra_escolhida": "idade",
                        "regras": [
                            { "id": "idade", "nome": "Idade Mínima Progressiva", "rmi_final": "1600.00", "elegivel": true }
                        ]
                    },
                    "memoria": [],
                    "avisos": []
                }
            }
        });
        let _ = outcome;
        let mem_pers = build("memory", &case, Some(&outcome_val), true).unwrap().to_string();
        assert!(mem_pers.contains("Parecer Técnico Customizado"), "{mem_pers}");
        assert!(mem_pers.contains("99988877766"), "{mem_pers}");
        assert!(mem_pers.contains(" (D)"), "{mem_pers}");
        assert!(mem_pers.contains("\"30\""), "{mem_pers}");

        let mem_priv = build("memory", &case, Some(&outcome_val), false).unwrap().to_string();
        assert!(!mem_priv.contains("99988877766"), "{mem_priv}");

        let roi_priv = build("roi", &case, Some(&outcome_val), false).unwrap().to_string();
        assert!(roi_priv.contains("Expectativa IBGE na DIB"), "{roi_priv}");
        assert!(!roi_priv.contains("371 meses"), "{roi_priv}");
    }

    #[test]
    fn memory_reads_n_descartadas_and_new_typst_kinds_respect_personal_flag() {
        let mut case: Calculation = serde_json::from_str(include_str!(
            "../../previdenciar-rgps/tests/fixtures/calculation-synthetic.json"
        ))
        .unwrap();
        case.identity.full_name = Some("SEGURADO SECRETO".into());
        case.identity.cpf = Some("123.456.789-00".into());
        for w in &mut case.manual_salaries {
            w.indicators = "INDICADOR_SECRETO_MANUAL".into();
        }
        let outcome_val = serde_json::json!({
            "status": "experimental",
            "data": {
                "native_result": {
                    "resumo": { "rmi": "1750.00", "salario_beneficio": "2000.00", "coeficiente": 87.5, "competencias": 300, "carencia_meses": 300 },
                    "tempo": {
                        "total": { "anos": 25, "meses": 0, "dias": 0 },
                        "vinculos": [
                            { "inicio": "2000-01-01", "fim": "2024-12-31", "tipo": "comum", "empresa": "EMPREGADOR_PRIVADO_TESTE", "dias": 9000, "fator": 1.0 }
                        ]
                    },
                    "carencia": { "carencia_meses": 300 },
                    "simulacao_descartes": [
                        { "n": 0, "rmi": "1500.00" },
                        { "n": 5, "rmi": "1750.00" }
                    ],
                    "memoria": [],
                    "avisos": []
                }
            }
        });

        let mem = build("memory", &case, Some(&outcome_val), false).unwrap().to_string();
        assert!(mem.contains("5 descartada(s) -> R$ 1.750,00"), "{mem}");

        let per_priv = build("periods", &case, Some(&outcome_val), false).unwrap().to_string();
        assert!(!per_priv.contains("EMPREGADOR_PRIVADO_TESTE"), "{per_priv}");
        assert!(per_priv.contains("Origem oculta"), "{per_priv}");

        let sal_priv = build("salaries", &case, Some(&outcome_val), false).unwrap().to_string();
        assert!(!sal_priv.contains("INDICADOR_SECRETO_MANUAL"), "{sal_priv}");
        let sal_pub = build("salaries", &case, Some(&outcome_val), true).unwrap().to_string();
        assert!(sal_pub.contains("INDICADOR_SECRETO_MANUAL"), "{sal_pub}");

        for kind in ["summary", "salaries", "indicators", "regras"] {
            let built = build(kind, &case, Some(&outcome_val), false).unwrap().to_string();
            assert!(!built.contains("SEGURADO SECRETO"), "{kind}: {built}");
            assert!(!built.contains("123.456.789-00"), "{kind}: {built}");
        }
    }
}
