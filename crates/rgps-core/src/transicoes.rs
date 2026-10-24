use crate::calendar::{idade_em_meses, IsoDate};
use crate::money::{add, div, mul, to_cents, Result};
use crate::rmi::coeficiente_progressivo;
use rust_decimal::Decimal;
use serde::Serialize;

pub const REFORMA: &str = "2019-11-13";
const TEMPO_MINIMO_MESES: [(&str, i64); 2] = [("feminino", 360), ("masculino", 420)];
const PONTOS_BASE: [(&str, i64); 2] = [("feminino", 86), ("masculino", 96)];
const PONTOS_TETO: [(&str, i64); 2] = [("feminino", 100), ("masculino", 105)];
const IDADE_BASE: [(&str, i64); 2] = [("feminino", 56), ("masculino", 61)];
const IDADE_TETO: [(&str, i64); 2] = [("feminino", 62), ("masculino", 65)];
const PEDAGIO_100_IDADE: [(&str, i64); 2] = [("feminino", 57), ("masculino", 60)];

fn kv(tabela: &[(&str, i64); 2], sexo: &str) -> i64 {
    tabela
        .iter()
        .find(|(s, _)| *s == sexo)
        .map(|(_, v)| *v)
        .unwrap_or(0)
}

pub fn pontos_exigidos(sexo: &str, ano: i32) -> i64 {
    let exigido = kv(&PONTOS_BASE, sexo) + (ano - 2019).max(0) as i64;
    exigido.min(kv(&PONTOS_TETO, sexo))
}

pub fn idade_exigida_meses(sexo: &str, ano: i32) -> i64 {
    let exigida = kv(&IDADE_BASE, sexo) as f64 + 0.5 * ((ano - 2019).max(0) as f64);
    (exigida.min(kv(&IDADE_TETO, sexo) as f64) * 12.0).round() as i64
}

/// Fator previdenciário (Lei 9.876/1999): (Tc×a/Es)×[1+(Id+Tc×a)/100],
/// com alíquota fixa de 0,31 e acréscimo de cinco anos ao tempo da mulher
/// (art. 29, § 9º, I, da Lei nº 8.213/1991, incluído pela Lei 9.876/1999).
/// O fator pode reduzir o salário de benefício; não há piso legal em 1.
pub fn fator_previdenciario(
    sexo: &str,
    tempo_total_meses: i64,
    idade_meses: i64,
    expectativa_sobrevida: Decimal,
) -> Result<Decimal> {
    let a = Decimal::from_str_exact("0.31").unwrap();
    let tempo_com_acrescimo = if sexo == "feminino" {
        tempo_total_meses + 60
    } else {
        tempo_total_meses
    };
    let tc = div(Decimal::from(tempo_com_acrescimo), Decimal::from(12))?;
    let id = div(Decimal::from(idade_meses), Decimal::from(12))?;
    let es = expectativa_sobrevida;
    if es <= Decimal::ZERO {
        return crate::money::erro(
            "Expectativa de sobrevida deve ser positiva para o fator previdenciário.",
        );
    }
    let parcela1 = div(mul(tc, a), es)?;
    let interna = add(add(id, mul(tc, a)), Decimal::from(100)) / Decimal::from(100);
    let fator = mul(parcela1, interna);
    Ok(fator)
}

#[derive(Debug, Clone, Serialize)]
pub struct RegraTransicao {
    pub id: &'static str,
    pub nome: String,
    pub fundamento: &'static str,
    pub elegivel: bool,
    /// Classificação do critério de RMI: regras-antigas | coeficiente-progressivo | 100%-media | fator-previdenciario
    pub criterio_rmi: &'static str,
    /// RMI estimada da regra (quando calculável), sem teto/piso.
    pub rmi: Option<String>,
    /// RMI da regra já com teto/piso aplicados.
    pub rmi_final: Option<String>,
    pub requisitos: Vec<String>,
    pub faltas: Option<serde_json::Value>,
    pub observacao: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransicoesResultado {
    pub referencia: String,
    pub sexo: String,
    pub idade_meses: i64,
    pub idade_anos: i64,
    pub tempo_total_meses: i64,
    pub tempo_total_anos: i64,
    pub tempo_ate_reforma_meses: i64,
    pub regras: Vec<RegraTransicao>,
    /// Regra vencedora (maior RMI entre as elegíveis calculáveis).
    pub regra_escolhida: Option<&'static str>,
    pub numero_regra_transicao: Option<u8>,
    pub rmi_escolhida: Option<String>,
    pub observacoes: Vec<String>,
}

pub struct TransicoesEntrada {
    pub sexo: String,
    pub data_nascimento: IsoDate,
    pub data_referencia: IsoDate,
    pub meses_ate_reforma: i64,
    pub meses_apos_reforma: i64,
    /// Salário de benefício (PBC) para as RMI das regras.
    pub salario_beneficio: Decimal,
    /// Teto/piso aplicados à RMI final da regra (opcional).
    pub teto: Option<Decimal>,
    pub piso: Option<Decimal>,
    /// Expectativa de sobrevida IBGE para o pedágio de 50% (opcional).
    pub expectativa_sobrevida: Option<Decimal>,
}

fn aplicar_limites(rmi: Decimal, teto: Option<Decimal>, piso: Option<Decimal>) -> (Decimal, Option<&'static str>) {
    if let Some(teto) = teto {
        if rmi > teto {
            return (teto, Some("teto"));
        }
    }
    if let Some(piso) = piso {
        if rmi < piso {
            return (piso, Some("piso"));
        }
    }
    (rmi, None)
}

/// Porta de buildTransicoes com extensão: RMI estimada por regra e seleção da vencedora.
pub fn build_transicoes(entrada: &TransicoesEntrada) -> Result<TransicoesResultado> {
    let sexo = entrada.sexo.as_str();
    let referencia = &entrada.data_referencia;
    let tempo_ate_reforma = entrada.meses_ate_reforma;
    let tempo_apos_reforma = entrada.meses_apos_reforma;
    let tempo_total = tempo_ate_reforma + tempo_apos_reforma;
    let idade_meses = idade_em_meses(&entrada.data_nascimento, referencia);
    let ano_referencia = referencia.ano;
    let tempo_minimo = kv(&TEMPO_MINIMO_MESES, sexo);

    let mut regras: Vec<RegraTransicao> = Vec::new();

    // 1. Direito adquirido (pré-reforma): elegibilidade apenas.
    let direito_adquirido = tempo_ate_reforma >= tempo_minimo;
    regras.push(RegraTransicao {
        id: "direito-adquirido",
        nome: "Direito adquirido (pré-reforma)".into(),
        fundamento: "Regras vigentes até 13/11/2019",
        elegivel: direito_adquirido,
        criterio_rmi: "regras-antigas",
        rmi: None,
        rmi_final: None,
        requisitos: vec![format!(
            "Tempo mínimo de {} anos cumprido até {REFORMA}",
            tempo_minimo / 12
        )],
        faltas: if direito_adquirido {
            None
        } else {
            Some(serde_json::json!({ "tempoMeses": tempo_minimo - tempo_ate_reforma }))
        },
        observacao: Some(
            "Regras antigas (média com descarte e fator previdenciário quando favorável) não são recalculadas neste comparador.".into(),
        ),
    });

    // 2. Pontos (art. 15).
    let pontos = pontos_exigidos(sexo, ano_referencia);
    let pontos_atuais = (tempo_total / 12) + (idade_meses / 12);
    let falta_tempo = (tempo_minimo - tempo_total).max(0);
    let falta_pontos = (pontos - pontos_atuais).max(0);
    let coeficiente = coeficiente_progressivo(sexo, tempo_total.max(0) as u32);
    let rmi_coeficiente = mul(entrada.salario_beneficio, div(coeficiente, Decimal::from(100))?);
    let (rmi_coef_final, limite_coef) = aplicar_limites(rmi_coeficiente, entrada.teto, entrada.piso);
    regras.push(RegraTransicao {
        id: "pontos",
        nome: format!("Transição por pontos ({pontos} pontos em {ano_referencia})"),
        fundamento: "EC 103/2019, art. 15",
        elegivel: falta_tempo == 0 && falta_pontos == 0,
        criterio_rmi: "coeficiente-progressivo",
        rmi: Some(crate::money::money(rmi_coeficiente)),
        rmi_final: Some(crate::money::money(rmi_coef_final)),
        requisitos: vec![
            format!("Tempo mínimo de {} anos", tempo_minimo / 12),
            format!(
                "Pontos exigidos em {ano_referencia}: {pontos} (idade {} + tempo {})",
                idade_meses / 12,
                tempo_total / 12
            ),
        ],
        faltas: if falta_tempo == 0 && falta_pontos == 0 {
            None
        } else {
            Some(serde_json::json!({ "tempoMeses": falta_tempo, "pontos": falta_pontos }))
        },
        observacao: limite_coef.map(|l| format!("Limite de {l} aplicado.")),
    });

    // 3. Idade progressiva (art. 16).
    let idade_exigida = idade_exigida_meses(sexo, ano_referencia);
    let falta_idade = (idade_exigida - idade_meses).max(0);
    regras.push(RegraTransicao {
        id: "idade-progressiva",
        nome: format!(
            "Transição por idade progressiva ({:.1} anos em {ano_referencia})",
            idade_exigida as f64 / 12.0
        ),
        fundamento: "EC 103/2019, art. 16",
        elegivel: falta_tempo == 0 && falta_idade == 0,
        criterio_rmi: "coeficiente-progressivo",
        rmi: Some(crate::money::money(rmi_coeficiente)),
        rmi_final: Some(crate::money::money(rmi_coef_final)),
        requisitos: vec![
            format!("Tempo mínimo de {} anos", tempo_minimo / 12),
            "Idade mínima progressiva".into(),
        ],
        faltas: if falta_tempo == 0 && falta_idade == 0 {
            None
        } else {
            Some(serde_json::json!({ "tempoMeses": falta_tempo, "idadeMeses": falta_idade }))
        },
        observacao: limite_coef.map(|l| format!("Limite de {l} aplicado.")),
    });

    // 4. Pedágio de 50% (art. 17): RMI por fator previdenciário.
    let faltava50 = (tempo_minimo - tempo_ate_reforma).max(0);
    let aplica50 = faltava50 > 0 && faltava50 <= 24;
    let pedagio50 = (faltava50 + 1) / 2;
    // Expectativa declarada pelo operador tem precedência; sem declaração, usa a
    // tábua IBGE embutida (vigência mais recente até a DER).
    let expectativa_tabelada = if entrada.expectativa_sobrevida.is_none() {
        let tabela = if sexo.eq_ignore_ascii_case("feminino") {
            &crate::dados::expectativa_sobrevida::EXPECTATIVA_FEMININA
        } else {
            &crate::dados::expectativa_sobrevida::EXPECTATIVA_MASCULINA
        };
        let idade = (idade_meses / 12).clamp(0, 90) as usize;
        let referencia = entrada.data_referencia.to_string();
        let mut linha_encontrada = None;
        for (vigencia, linha) in tabela.iter() {
            if *vigencia <= referencia.as_str() { linha_encontrada = Some(linha); } else { break; }
        }
        linha_encontrada.and_then(|linha| {
            let valor = linha[idade];
            if valor.is_empty() { None } else { Decimal::from_str_exact(valor).ok() }
        })
    } else {
        None
    };
    let fonte_expectativa = if entrada.expectativa_sobrevida.is_some() { "declarada pelo operador" } else { "da tábua IBGE embutida" };
    let (rmi_fator, rmi_fator_final, observacao_fator) = match entrada.expectativa_sobrevida.or(expectativa_tabelada) {
        Some(es) if aplica50 => {
            let idade_der = idade_meses;
            let fator = fator_previdenciario(sexo, tempo_total, idade_der, es)?;
            let bruta = mul(entrada.salario_beneficio, fator);
            let (final_valor, limite) = aplicar_limites(bruta, entrada.teto, entrada.piso);
            (
                Some(bruta),
                Some(final_valor),
                Some(format!(
                    "Fator previdenciário {:.4} aplicado (expectativa {}){}.",
                    fator,
                    fonte_expectativa,
                    limite.map(|l| format!("; limite de {l} aplicado")).unwrap_or_default()
                )),
            )
        }
        _ => (None, None, Some(
            "O RMI desta regra usa fator previdenciário (Lei 9.876/1999); a tábua embutida não cobre a idade/data desta DER. Informe a expectativa de sobrevida do IBGE para calculá-lo.".into(),
        )),
    };
    regras.push(RegraTransicao {
        id: "pedagio-50",
        nome: "Transição por pedágio de 50%".into(),
        fundamento: "EC 103/2019, art. 17",
        elegivel: aplica50 && tempo_total >= tempo_minimo + pedagio50,
        criterio_rmi: "fator-previdenciario",
        rmi: rmi_fator.map(crate::money::money),
        rmi_final: rmi_fator_final.map(crate::money::money),
        requisitos: vec![
            "Faltavam até 2 anos em 13/11/2019".into(),
            format!("Pedágio de {pedagio50} mês(es)"),
        ],
        faltas: Some(if !aplica50 {
            serde_json::json!({ "aplicavel": false })
        } else if tempo_total >= tempo_minimo + pedagio50 {
            serde_json::Value::Null
        } else {
            serde_json::json!({ "tempoMeses": tempo_minimo + pedagio50 - tempo_total })
        }),
        observacao: observacao_fator,
    });

    // 5. Pedágio de 100% (art. 20): RMI de 100% da média.
    let aplica100 = tempo_ate_reforma > 0 && tempo_ate_reforma < tempo_minimo;
    let pedagio100 = faltava50;
    let idade_100_exigida = kv(&PEDAGIO_100_IDADE, sexo) * 12;
    let falta_idade_100 = (idade_100_exigida - idade_meses).max(0);
    let falta_tempo_100 = (tempo_minimo + pedagio100 - tempo_total).max(0);
    let rmi_media = to_cents(entrada.salario_beneficio);
    let (rmi_media_final, limite_media) = aplicar_limites(rmi_media, entrada.teto, entrada.piso);
    regras.push(RegraTransicao {
        id: "pedagio-100",
        nome: "Transição por pedágio de 100%".into(),
        fundamento: "EC 103/2019, art. 20",
        elegivel: aplica100 && falta_idade_100 == 0 && falta_tempo_100 == 0,
        criterio_rmi: "100%-media",
        rmi: Some(crate::money::money(rmi_media)),
        rmi_final: Some(crate::money::money(rmi_media_final)),
        requisitos: vec![
            format!("Idade mínima de {} anos", kv(&PEDAGIO_100_IDADE, sexo)),
            format!("Tempo mínimo de {} anos", tempo_minimo / 12),
            format!("Pedágio de {pedagio100} mês(es)"),
        ],
        faltas: Some(if !aplica100 {
            serde_json::json!({ "aplicavel": false })
        } else if falta_idade_100 == 0 && falta_tempo_100 == 0 {
            serde_json::Value::Null
        } else {
            serde_json::json!({ "tempoMeses": falta_tempo_100, "idadeMeses": falta_idade_100 })
        }),
        observacao: Some(format!(
            "RMI de 100% da média, sem redutor{}.",
            limite_media.map(|l| format!("; limite de {l} aplicado")).unwrap_or_default()
        )),
    });

    // 6. Idade permanente (art. 18).
    let idade_regra_geral = if sexo == "feminino" { 62 * 12 } else { 65 * 12 };
    let falta_idade_geral = (idade_regra_geral - idade_meses).max(0);
    let falta_carencia = (180 - tempo_total).max(0);
    regras.push(RegraTransicao {
        id: "idade-permanente",
        nome: format!("Regra permanente por idade ({})", idade_regra_geral / 12),
        fundamento: "EC 103/2019, art. 18",
        elegivel: falta_idade_geral == 0 && falta_carencia == 0,
        criterio_rmi: "coeficiente-progressivo",
        rmi: Some(crate::money::money(rmi_coeficiente)),
        rmi_final: Some(crate::money::money(rmi_coef_final)),
        requisitos: vec!["Idade mínima".into(), "Carência de 15 anos".into()],
        faltas: if falta_idade_geral == 0 && falta_carencia == 0 {
            None
        } else {
            Some(serde_json::json!({
                "idadeMeses": falta_idade_geral,
                "carenciaMeses": falta_carencia
            }))
        },
        observacao: limite_coef.map(|l| format!("Limite de {l} aplicado.")),
    });

    // Vencedora: maior RMI final entre elegíveis com RMI calculada.
    let mut vencedora: Option<(&'static str, u8, Decimal)> = None;
    for (indice, regra) in regras.iter().enumerate() {
        if !regra.elegivel || regra.rmi_final.is_none() {
            continue;
        }
        let valor = crate::money::parse_money(regra.rmi_final.as_ref().unwrap(), "RMI da regra")?;
        if vencedora.is_none() || valor > vencedora.as_ref().unwrap().2 {
            vencedora = Some((regra.id, indice as u8 + 1, valor));
        }
    }

    Ok(TransicoesResultado {
        referencia: referencia.to_string(),
        sexo: sexo.into(),
        idade_meses,
        idade_anos: idade_meses / 12,
        tempo_total_meses: tempo_total,
        tempo_total_anos: tempo_total / 12,
        tempo_ate_reforma_meses: tempo_ate_reforma,
        regra_escolhida: vencedora.map(|(id, _, _)| id),
        numero_regra_transicao: vencedora.map(|(_, numero, _)| numero),
        rmi_escolhida: vencedora.map(|(_, _, v)| crate::money::money(v)),
        regras,
        observacoes: vec![
            "Tempo apurado por competências: não considera concomitância, tempo especial, rural ou militar; confira o CNIS.".into(),
            "A RMI de cada regra é estimativa com o PBC atual e limites informados; a regra vencedora é a de maior RMI entre as elegíveis.".into(),
            "Pedágio de 50% exige expectativa de sobrevida do IBGE (informada pelo operador, como no ATRIUM).".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::parse_money;

    fn entrada(sexo: &str, nascimento: &str, referencia: &str, ate_reforma: i64, apos: i64) -> TransicoesEntrada {
        TransicoesEntrada {
            sexo: sexo.into(),
            data_nascimento: IsoDate::parse(nascimento, "t").unwrap(),
            data_referencia: IsoDate::parse(referencia, "t").unwrap(),
            meses_ate_reforma: ate_reforma,
            meses_apos_reforma: apos,
            salario_beneficio: parse_money("2000.00", "sb").unwrap(),
            teto: None,
            piso: None,
            expectativa_sobrevida: None,
        }
    }

    #[test]
    fn homen_420_meses_pontos_e_idade_permanente() {
        // Homem com 35 anos de tempo, nascido em 1958-03-10, DER 2026-10-09
        // (idade 68a6m + tempo 35a = 103 pontos em 2026, exatamente o exigido).
        let resultado = build_transicoes(&entrada(
            "masculino",
            "1958-03-10",
            "2026-10-09",
            420,
            0,
        ))
        .unwrap();
        let pontos = resultado.regras.iter().find(|r| r.id == "pontos").unwrap();
        assert!(pontos.elegivel, "pontos: {:?}", pontos);
        let permanente = resultado
            .regras
            .iter()
            .find(|r| r.id == "idade-permanente")
            .unwrap();
        assert!(permanente.elegivel);
        let pedagio50 = resultado
            .regras
            .iter()
            .find(|r| r.id == "pedagio-50")
            .unwrap();
        assert_eq!(
            pedagio50.faltas.as_ref().unwrap().get("aplicavel").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert!(resultado.regra_escolhida.is_some());
        assert!(resultado.numero_regra_transicao.unwrap() >= 2);
    }

    #[test]
    fn fator_usa_aliquota_031_e_acrescimo_de_cinco_anos_para_mulher() {
        let masculino = fator_previdenciario("masculino", 120, 600, Decimal::from(40)).unwrap();
        let feminino = fator_previdenciario("feminino", 120, 600, Decimal::from(40)).unwrap();
        // Lei 9.876/1999: a = 0,31 (homem) e 0,30 (mulher).
        assert_eq!(masculino, Decimal::from_str_exact("0.1186525").unwrap());
        // Mulher: Tc = 10 anos + 5 = 15 anos; 15×0,31/40 × [1+(50+4,65)/100] = 0,179780625.\n        assert_eq!(feminino, Decimal::from_str_exact("0.179780625").unwrap());
    }

    #[test]
    fn pedagio_50_calcula_com_expectativa() {
        let mut entrada = entrada("feminino", "1963-01-15", "2026-10-09", 348, 24);
        entrada.expectativa_sobrevida = Some(parse_money("18.5", "es").unwrap());
        let resultado = build_transicoes(&entrada).unwrap();
        let pedagio50 = resultado.regras.iter().find(|r| r.id == "pedagio-50").unwrap();
        // faltava 12 meses na reforma; pedágio 6 meses; tempo total 372 >= 360+6.
        assert!(pedagio50.elegivel, "{pedagio50:?}");
        assert!(pedagio50.rmi.is_some());
    }

    #[test]
    fn pedagio_50_usa_tabua_embutida_sem_expectativa_declarada() {
        let entrada = entrada("feminino", "1963-01-15", "2026-10-09", 348, 24);
        assert!(entrada.expectativa_sobrevida.is_none());
        let resultado = build_transicoes(&entrada).unwrap();
        let pedagio50 = resultado.regras.iter().find(|r| r.id == "pedagio-50").unwrap();
        assert!(pedagio50.rmi.is_some(), "{pedagio50:?}");
        assert!(pedagio50
            .observacao
            .as_deref()
            .unwrap_or("")
            .contains("tábua IBGE"));
    }

    #[test]
    fn coeficiente_progressivo_60_2() {
        let coeficiente = coeficiente_progressivo("feminino", 300);
        assert_eq!(coeficiente, Decimal::from(80));
    }
}
