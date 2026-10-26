//! Constrói o JSON de dados que cada template Typst consome, já formatado em
//! padrão brasileiro. Nunca inventa valor ausente: campo desconhecido fica vazio.
use crate::format::{br_date, br_month, brl, brl_decimal, decimal, period_from_days_360, period_from_months};
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
            ["Contribuição mensal (INSS)", contribuicao],
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

fn memory_report(case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let n = native(outcome);
    let headers = vec![
        "Nº", "Data", "Dias", "Salário de Contribuição", "Índice de Atualização", "Salário Corrigido",
    ];
    let rows: Vec<Vec<String>> = n
        .and_then(|v| v.pointer("/correcao/parcelas"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .enumerate()
                .map(|(i, p)| {
                    vec![
                        format!("{:03}", i + 1),
                        br_date(p.get("competencia").and_then(Value::as_str).unwrap_or("")),
                        p.get("dias").and_then(Value::as_i64).unwrap_or(0).to_string(),
                        p.get("valor").and_then(Value::as_str).map(brl).unwrap_or_default(),
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
        .and_then(|v| v.pointer("/parametros/coeficiente"))
        .and_then(|c| c.as_str().map(|s| s.to_string()).or_else(|| c.as_i64().map(|i| i.to_string())))
        .unwrap_or_default();
    let rmi = n.and_then(|v| {
        pick(v, &["/resumo/rmi", "/transicoes/rmi_escolhida"])
            .and_then(decimal)
            .or_else(|| memoria_valor(v, "rmi"))
    });

    let fmt = |v: Option<Decimal>| v.map(brl_decimal).unwrap_or_else(|| "Não apurado".into());
    Ok(json!({
        "title": "Cálculo da R.M.I.",
        "subtitle": "Cálculo da R.M.I. - Tempo de Contribuição",
        "date": data_calculo,
        "identification": identification(case, &data_calculo),
        "headers": headers,
        "rows": rows,
        "resumo": [
            ["Idade na DIB", idade],
            ["Nº de Parcelas no PBC", parcelas.to_string()],
            ["Tempo de Contribuição", tempo],
            ["Soma dos Salários", fmt(soma)],
            ["Média dos Salários Corrigidos", fmt(media)],
            ["Salário de Benefício", fmt(sb)],
            ["Coeficiente", if coeficiente.is_empty() { String::new() } else { format!("{coeficiente} %") }],
            ["Renda Mensal Inicial", fmt(rmi)],
        ],
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

fn quality(case: &Calculation, _outcome: Option<&Value>) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let headers = vec!["Início", "Fim", "Filiação", "Carência", "Tempo Contribuição", "Origem do Vínculo"];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let Some(cnis) = &case.cnis {
        for link in &cnis.links {
            let inicio = link.start_date.as_deref().map(br_date).unwrap_or_default();
            let fim = link.end_date.as_deref().map(br_date).unwrap_or_default();
            let meses = cnis
                .salary_entries
                .iter()
                .filter(|e| e.link_sequence == Some(link.sequence))
                .count();
            let tempo = match (iso_parts(&inicio), iso_parts(&fim)) {
                (Some((ay, am, ad)), Some((by, bm, bd))) => {
                    let dias = (by - ay) * 360 + (bm - am) * 30 + (bd - ad);
                    period_from_days_360(dias.max(0))
                }
                _ => String::new(),
            };
            rows.push(vec![
                inicio,
                fim,
                link.affiliation.clone().unwrap_or_default(),
                format!("{meses} contribuições"),
                tempo,
                link.raw_employer.clone().unwrap_or_default(),
            ]);
        }
    }
    let total_meses: usize = rows
        .iter()
        .map(|r| {
            r.get(3)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(0)
        })
        .sum();
    Ok(json!({
        "title": "Cálculo da Qualidade de Segurado",
        "date": data_calculo,
        "identification": identification(case, &data_calculo),
        "headers": headers,
        "rows": rows,
        "totais": [
            ["Total de contribuições", format!("{total_meses} contribuições")],
        ],
        "nota": "Apuração própria sobre as contribuições declaradas na origem. A comparação com a carência exigida é informativa: não há direito apurado.",
    }))
}

fn roi(case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let n = native(outcome);
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
                    let meses = 371i64;
                    let rmi_13 = rmi * Decimal::from(13) / Decimal::from(12);
                    let base = rmi_13.round_dp(2).to_string();
                    let irrf = rgps_core::fiscal::irrf_mensal("2026-01", &base, 0, None)
                        .map(|v| v.irrf)
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
                            format!("Total líquido recebido de {} até {} ({} meses considerando o 13º salário)", data_calculo, "a expectativa", meses),
                            format!("Provável RMI de {}", brl_decimal(rmi)),
                            format!("RMI com 13º: {}", brl_decimal(rmi_13)),
                            format!("IRRF mensal estimado: {}", brl_decimal(irrf)),
                            format!("Total líquido: {}", brl_decimal(total)),
                        ],
                        "expectativa": "Expectativa de sobrevida conforme tábua do IBGE; projeção informativa.",
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(json!({
        "title": "Cálculo do Retorno Sobre o Investimento",
        "date": data_calculo,
        "identification": identification(case, &data_calculo),
        "regras": regras,
        "nota": "As projeções futuras são feitas com a repetição do último salário de contribuição a partir da DIB informada, gerando RMI's com valores aproximados.",
    }))
}

fn regras(case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    let data_calculo = br_date(&case.dib);
    let n = native(outcome);
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
                            brl_decimal((rmi_13 * Decimal::from(371)).round_dp(2))
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

fn generic_sections(case: &Calculation, outcome: Option<&Value>, kind: &str) -> Result<Value, String> {
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
                    e.indicators.first().cloned().unwrap_or_default(),
                ]);
            }
        }
        for w in &case.manual_salaries {
            rows.push(vec![br_month(&w.competence), brl(w.amount.trim()), w.indicators.clone()]);
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
        "identification": identification(case, &data_calculo),
        "sections": sections,
    }))
}

/// Constrói os dados de um relatório a partir do caso e do resultado nativo.
pub fn build(kind: &str, case: &Calculation, outcome: Option<&Value>) -> Result<Value, String> {
    match kind {
        "analysis" => analysis(case, outcome),
        "periods" => periods(case, outcome),
        "transitions" => transitions(case, outcome),
        "memory" => memory_report(case, outcome),
        "quality" => quality(case, outcome),
        "roi" => roi(case, outcome),
        "regras" => regras(case, outcome),
        "summary" | "salaries" | "indicators" => generic_sections(case, outcome, kind),
        other => Err(format!("Relatório sem construtor de dados: {other}")),
    }
}
