use crate::atualizacao::teto_vigente;
use crate::calendar::{Competencia, IsoDate};
use crate::carencia::calcular_carencia;
use crate::especie::{self, Especie};
use crate::money::{add, div, money, mul, parse_money, Result, RgpsError};
use crate::rmi::{
    calcular_pbc_com_correcao, divisor_usado, salario_beneficio_sem_correcao,
};
use crate::tempo::{calcular_tempo, VinculoEntrada};
use crate::transicoes::{build_transicoes, TransicoesEntrada};
use rust_decimal::Decimal;
use serde::Serialize;

const ANOS_BASE: [(&str, i64); 2] = [("feminino", 15), ("masculino", 20)];
const COMPETENCIA_REFORMA: Competencia = Competencia { ano: 2019, mes: 11 };

#[derive(Debug, Clone)]
pub struct RgpsEntrada {
    pub nome: Option<String>,
    /// Espécie INSS declarada. Só a 42 (aposentadoria por tempo de
    /// contribuição) tem motor aqui: outra espécie bloqueia, nunca é aproximada.
    pub especie: Option<String>,
    pub sexo: String,
    pub data_nascimento: Option<IsoDate>,
    pub data_referencia: Option<IsoDate>,
    pub vinculos: Vec<VinculoEntrada>,
    pub competencias: Vec<(Competencia, Decimal)>,
    pub opcoes: RgpsOpcoes,
}

#[derive(Debug, Clone, Default)]
pub struct RgpsOpcoes {
    pub corrigir_salarios: bool,
    pub descarte: bool,
    pub divisor_minimo: bool,
    /// Descarte dos menores salários (EC 103/2019, art. 26, § 6º) mantendo o
    /// mínimo de competências declarado como necessário para o direito.
    pub descartar_menores_minimo: Option<u32>,
    pub referencia_correcao: Option<Competencia>,
    pub teto: Option<Decimal>,
    pub piso: Option<Decimal>,
    /// Aplica o teto vigente do salário de contribuição na competência da DER.
    pub aplicar_teto_vigente_der: bool,
    pub expectativa_sobrevida: Option<Decimal>,
    pub periodos_graca: Vec<(IsoDate, IsoDate)>,
    pub recolhimentos_atraso: Vec<Competencia>,
    pub regra_alvo: Option<String>,
    /// Origem declarada da incapacidade (31/32): comum, acidente,
    /// doenca_ocupacional, doenca_do_trabalho ou doenca_listada.
    pub incapacidade_origem: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoriaItem {
    pub tipo: &'static str,
    pub descricao: String,
    pub valor: String,
}

#[derive(Debug, Serialize)]
pub struct RgpsResultado {
    pub segurado: SeguradoView,
    pub especie: EspecieView,
    pub primeira_competencia: String,
    pub ultima_competencia: String,
    pub parametros: ParametrosView,
    pub resumo: ResumoView,
    pub tempo: Option<serde_json::Value>,
    pub carencia: serde_json::Value,
    pub correcao: Option<serde_json::Value>,
    pub memoria: Vec<MemoriaItem>,
    pub transicoes: Option<serde_json::Value>,
    /// Requisitos e regras próprias das espécies 41, 32 e 31.
    pub beneficio: Option<serde_json::Value>,
    pub avisos: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EspecieView {
    pub codigo: String,
    pub nome: String,
    pub regra_escolhida: Option<String>,
    /// Nome legível da regra escolhida (41/32/31); a 42 usa o comparador de transições.
    pub regra_nome: Option<String>,
    pub elegivel: bool,
}

#[derive(Debug, Serialize)]
pub struct SeguradoView {
    pub nome: Option<String>,
    pub sexo: String,
    pub data_nascimento: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParametrosView {
    pub divisor: u32,
    pub divisor_minimo_aplicado: bool,
    pub anos: i64,
    pub meses_restantes: i64,
    pub anos_base: i64,
    pub coeficiente: String,
    pub correcao_ativa: bool,
    pub referencia_correcao: Option<String>,
    pub descarte_aplicado: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ResumoView {
    pub competencias: usize,
    pub carencia_meses: usize,
    pub soma_salarios: String,
    pub salario_beneficio: String,
    pub coeficiente: String,
    pub rmi_bruta: String,
    pub rmi: String,
    pub limite_aplicado: Option<&'static str>,
    pub teto: Option<String>,
    pub piso: Option<String>,
}

fn anos_base(sexo: &str) -> i64 {
    ANOS_BASE
        .iter()
        .find(|(s, _)| *s == sexo)
        .map(|(_, v)| *v)
        .unwrap_or(20)
}

/// Média dos últimos 12 salários de contribuição, corrigidos quando o PBC foi
/// corrigido. Usada no limite do auxílio por incapacidade temporária
/// (Lei 8.213/1991, art. 29, § 10); não substitui o salário de benefício.
fn media_ultimos_12(
    competencias: &[(Competencia, Decimal)],
    correcao: Option<&crate::rmi::PbcResultado>,
) -> Option<Decimal> {
    let valores: Vec<Decimal> = match correcao {
        Some(pbc) => {
            let mut parcelas: Vec<&crate::rmi::ParcelaPbc> = pbc.parcelas.iter().collect();
            parcelas.sort_by(|a, b| a.competencia.cmp(&b.competencia));
            parcelas
                .iter()
                .rev()
                .take(12)
                .filter_map(|p| parse_money(&p.corrigido, "média dos últimos 12").ok())
                .collect()
        }
        None => competencias
            .iter()
            .rev()
            .take(12)
            .map(|(_, valor)| *valor)
            .collect(),
    };
    if valores.is_empty() {
        return None;
    }
    let total = valores.iter().copied().fold(Decimal::ZERO, add);
    div(total, Decimal::from(valores.len() as u64)).ok()
}

/// Valida a entrada (porta de validate() do previdenciario.mjs).
pub fn validar(entrada: &RgpsEntrada) -> Result<()> {
    if !crate::SEXOS.contains(&entrada.sexo.as_str()) {
        return Err(RgpsError(format!("Sexo inválido: {}.", entrada.sexo)));
    }
    if entrada.competencias.is_empty() {
        return Err(RgpsError(
            "Informe ao menos uma competência com salário de contribuição.".into(),
        ));
    }
    if let Some(especie) = entrada.especie.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Especie::parse(Some(especie))?;
    }
    let mut vistas: std::collections::BTreeSet<Competencia> = std::collections::BTreeSet::new();
    let primeira = Competencia::parse(crate::PRIMEIRA_COMPETENCIA, "primeira competência")?;
    for (competencia, salario) in &entrada.competencias {
        if *competencia < primeira {
            return Err(RgpsError(format!(
                "Competência {competencia} é anterior a {}; períodos anteriores não entram no PBC.",
                crate::PRIMEIRA_COMPETENCIA
            )));
        }
        if !vistas.insert(*competencia) {
            return Err(RgpsError(format!(
                "Competência duplicada: {competencia}. Concomitância exige conferência manual."
            )));
        }
        if *salario < Decimal::ZERO {
            return Err(RgpsError(format!(
                "Salário da competência {competencia} não pode ser negativo."
            )));
        }
        if salario.is_zero() {
            return Err(RgpsError(format!(
                "Salário da competência {competencia} deve ser maior que zero."
            )));
        }
    }
    if let (Some(teto), Some(piso)) = (entrada.opcoes.teto, entrada.opcoes.piso) {
        if piso > teto {
            return Err(RgpsError("Piso informado é maior que o teto.".into()));
        }
    }
    Ok(())
}

/// Cálculo RGPS completo: valida, apura tempo, carência, PBC, coeficiente,
/// RMI com limites, transições EC 103 com RMI por regra e vencedora.
pub fn calcular(entrada: &RgpsEntrada) -> Result<RgpsResultado> {
    validar(entrada)?;
    let especie = Especie::parse(entrada.especie.as_deref())?;

    let mut competencias = entrada.competencias.clone();
    competencias.sort_by_key(|(c, _)| *c);
    let quantidade = competencias.len();
    let primeira = competencias.first().unwrap().0;
    let ultima = competencias.last().unwrap().0;

    // Tempo de contribuição (opcional).
    let tempo = if entrada.vinculos.is_empty() {
        None
    } else {
        Some(calcular_tempo(&entrada.vinculos, entrada.data_referencia.as_ref())?)
    };
    let tempo_meses = tempo
        .as_ref()
        .map(|t| (t.marcos.ate_ec103 + t.marcos.apos_ec103) / 30)
        .unwrap_or(0);
    let inicio_filiacao = entrada
        .vinculos
        .iter()
        .map(|v| v.inicio)
        .min()
        .or_else(|| {
            competencias.first().map(|(c, _)| IsoDate {
                ano: c.ano as i32,
                mes: c.mes,
                dia: 1,
            })
        });
    // Só trata como rural quando TODOS os vínculos são rural/especial; mistura
    // exige análise própria e não é presumida.
    let rural = !entrada.vinculos.is_empty()
        && entrada
            .vinculos
            .iter()
            .all(|v| matches!(v.tipo.as_str(), "rural" | "especial"));

    // Carência qualificada: salários ∪ vínculos ∪ graça ∪ atraso.
    let vinculos_ranges: Vec<(IsoDate, IsoDate)> = entrada
        .vinculos
        .iter()
        .map(|v| {
            let fim = v
                .fim
                .or(entrada.data_referencia)
                .ok_or_else(|| RgpsError("Vínculo sem fim exige data de referência.".into()))?;
            Ok((v.inicio, fim))
        })
        .collect::<Result<Vec<_>>>()?;
    let carencia = calcular_carencia(
        &competencias.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        &vinculos_ranges,
        &entrada.opcoes.periodos_graca,
        &entrada.opcoes.recolhimentos_atraso,
    )?;

    // PBC / salário de benefício.
    let correcao_ativa = entrada.opcoes.corrigir_salarios;
    let correcao = if correcao_ativa {
        let referencia_correcao = entrada.opcoes.referencia_correcao.unwrap_or(ultima);
        Some(match entrada.opcoes.descartar_menores_minimo {
            Some(minimo) => crate::rmi::calcular_pbc_com_descarte_minimo(
                &competencias,
                &referencia_correcao,
                minimo,
            )?,
            None => calcular_pbc_com_correcao(
                &competencias,
                &referencia_correcao,
                entrada.opcoes.descarte,
            )?,
        })
    } else {
        None
    };

    let salario_beneficio = match &correcao {
        Some(pbc) => parse_money(&pbc.salario_beneficio, "salário de benefício")?,
        None => salario_beneficio_sem_correcao(&competencias, entrada.opcoes.divisor_minimo)?,
    };

    // Coeficiente: a espécie 42 mantém a paridade por competências; 41, 32 e 31
    // usam a regra própria (idade/incapacidade/91%), com requisitos explícitos.
    let media_12 = media_ultimos_12(&competencias, correcao.as_ref());
    let avaliacao = especie::avaliar(&especie::AvaliacaoEntrada {
        especie,
        sexo: &entrada.sexo,
        data_nascimento: entrada.data_nascimento,
        data_referencia: entrada.data_referencia,
        carencia_meses: carencia.carencia_meses as i64,
        tempo_meses,
        competencias: quantidade as i64,
        inicio_filiacao,
        rural,
        incapacidade_origem: entrada.opcoes.incapacidade_origem.as_deref(),
        media_ultimos_12: media_12,
    })?;
    let anos = (quantidade / 12) as i64;
    let meses_restantes = (quantidade % 12) as i64;
    let base = anos_base(&entrada.sexo);
    let excedente = (anos - base).max(0);
    let coeficiente = if especie == Especie::TempoContribuicao {
        Decimal::from(60) + Decimal::from(2) * Decimal::from(excedente)
    } else {
        avaliacao.coeficiente
    };

    let mut rmi_bruta = mul(
        salario_beneficio,
        div(coeficiente, Decimal::from(100))?,
    );
    let mut limite_aplicado: Option<&'static str> = None;

    // Espécie 31: a RMI não pode exceder a média dos últimos 12 salários
    // (Lei 8.213/1991, art. 29, § 10).
    if avaliacao.limite_media_12 {
        if let Some(media) = media_12 {
            if rmi_bruta > media {
                rmi_bruta = media;
                limite_aplicado = Some("media_12");
            }
        }
    }

    let teto_derivado = if entrada.opcoes.aplicar_teto_vigente_der {
        Some(teto_vigente(
            &entrada
                .opcoes
                .referencia_correcao
                .unwrap_or(ultima),
        )?)
    } else {
        entrada.opcoes.teto
    };
    if let Some(teto) = teto_derivado {
        if rmi_bruta > teto {
            rmi_bruta = teto;
            limite_aplicado = Some("teto");
        }
    }
    if let Some(piso) = entrada.opcoes.piso {
        if limite_aplicado.is_none() && rmi_bruta < piso {
            rmi_bruta = piso;
            limite_aplicado = Some("piso");
        }
    }

    // Transições EC 103 (quando há nascimento).
    let mut avisos = Vec::new();
    let mut transicoes = None;
    if let Some(nascimento) = entrada.data_nascimento {
        let data_referencia = entrada.data_referencia.ok_or_else(|| {
            RgpsError(
                "Informe a data de referência (DER) para comparar as regras de transição.".into(),
            )
        })?;
        // Tempo apurado (dias efetivos / 30) antes e depois de 13/11/2019.
        // A contagem de competências fica só como fallback quando não há vínculos.
        let (meses_ate_reforma, meses_apos_reforma) = match tempo.as_ref() {
            Some(t) => (t.marcos.ate_ec103 / 30, t.marcos.apos_ec103 / 30),
            None => {
                let ate = competencias
                    .iter()
                    .filter(|(c, _)| *c <= COMPETENCIA_REFORMA)
                    .count() as i64;
                (ate, quantidade as i64 - ate)
            }
        };
        let resultado = build_transicoes(&TransicoesEntrada {
            sexo: entrada.sexo.clone(),
            data_nascimento: nascimento,
            data_referencia,
            meses_ate_reforma,
            meses_apos_reforma,
            salario_beneficio,
            teto: teto_derivado,
            piso: entrada.opcoes.piso,
            expectativa_sobrevida: entrada.opcoes.expectativa_sobrevida,
        })?;
        transicoes = Some(serde_json::to_value(&resultado).expect("serialização de transições"));
    } else {
        avisos.push(
            "Sem data de nascimento informada, as regras de transição da EC 103 não são comparadas.".into(),
        );
    }

    // Memória auditável.
    let divisor = divisor_usado(quantidade, entrada.opcoes.divisor_minimo);
    let soma_total: Decimal = competencias.iter().map(|(_, s)| *s).fold(Decimal::ZERO, add);
    let mut memoria = vec![MemoriaItem {
        tipo: "media",
        descricao: match &correcao {
            Some(pbc) => format!(
                "Média dos {} maiores salários corrigidos (PBC de {}; {} descartados)",
                pbc.consideradas, pbc.pbc, pbc.descartadas
            ),
            None => format!("Soma de {quantidade} competências dividida por {divisor}"),
        },
        valor: money(salario_beneficio),
    }];
    memoria.push(MemoriaItem {
        tipo: "coeficiente",
        descricao: match especie {
            Especie::TempoContribuicao => format!(
                "60% + 2% × {excedente} ano(s) excedente(s) a {base} (por competências informadas)"
            ),
            Especie::Idade | Especie::IncapacidadePermanente => format!(
                "Coeficiente da espécie {}: {coeficiente}% sobre a média",
                especie.codigo()
            ),
            Especie::IncapacidadeTemporaria => {
                "91% do salário de benefício (Lei 8.213/1991, art. 61)".into()
            }
        },
        valor: format!("{coeficiente}%"),
    });
    memoria.push(MemoriaItem {
        tipo: "rmi",
        descricao: "Salário de benefício × coeficiente".into(),
        valor: money(rmi_bruta),
    });
    if let Some(limite) = limite_aplicado {
        memoria.push(MemoriaItem {
            tipo: "limite",
            descricao: match limite {
                "teto" => "Teto aplicado".into(),
                "piso" => "Piso aplicado".into(),
                "media_12" => {
                    "Limite da média dos últimos 12 salários (Lei 8.213/1991, art. 29, § 10)"
                        .into()
                }
                outro => format!("Limite {outro} aplicado"),
            },
            valor: money(rmi_bruta),
        });
    }

    // Avisos (paridade com o ATRIUM + extensões deste motor).
    avisos.push(if correcao_ativa {
        "Salários nominais corrigidos pelo INPC com o teto da época aplicado antes da correção.".into()
    } else {
        "Informe os salários de contribuição já corrigidos para a mesma data de referência; a correção monetária é etapa separada.".into()
    });
    avisos.push(
        "Tempo especial, rural e concomitância exigem análise própria; confira o CNIS e a carta de concessão antes de usar o resultado.".into(),
    );
    if correcao_ativa {
        avisos.push(
            "O descarte aplicado é o dos 20% menores salários corrigidos (regra pré-reforma); o descarte do art. 26, § 6º, da EC 103 não é aplicado.".into(),
        );
        avisos.push(
            "Competências anteriores a 2001 podem divergir da tabela de transição oficial do INSS; o descarte dos 20% menores costuma absorver a diferença.".into(),
        );
    }
    if especie == Especie::TempoContribuicao && anos < base {
        avisos.push(format!(
            "Tempo informado ({anos} anos) abaixo do mínimo de {base} anos para o coeficiente cheio; o coeficiente permanece em 60%."
        ));
    }
    if entrada.opcoes.divisor_minimo && (quantidade as u32) < crate::DIVISOR_MINIMO {
        avisos.push(format!(
            "Divisor mínimo aplicado: a soma foi dividida por {} (Lei 14.331/2022), e não por {quantidade} competências.",
            crate::DIVISOR_MINIMO
        ));
    }
    if entrada.opcoes.aplicar_teto_vigente_der {
        avisos.push(
            "Teto vigente do salário de contribuição aplicado automaticamente na competência de referência.".into(),
        );
    }
    if especie != Especie::TempoContribuicao {
        let regra_ref = avaliacao
            .regras
            .iter()
            .find(|r| Some(r.id) == avaliacao.regra_escolhida)
            .or_else(|| avaliacao.regras.iter().min_by_key(|r| r.faltas_meses));
        memoria.push(MemoriaItem {
            tipo: "especie",
            descricao: format!("Espécie {} — {}", avaliacao.especie, avaliacao.nome),
            valor: regra_ref
                .map(|r| r.nome.clone())
                .unwrap_or_else(|| "requisitos não cumpridos".into()),
        });
        if let Some(regra) = regra_ref {
            for req in &regra.requisitos {
                memoria.push(MemoriaItem {
                    tipo: "requisito",
                    descricao: format!(
                        "{} — {}: {} (exigido: {}); {}",
                        regra.nome,
                        req.descricao,
                        req.apurado,
                        req.exigido,
                        if req.cumprido { "cumprido" } else { "não cumprido" }
                    ),
                    valor: req
                        .falta_meses
                        .filter(|m| *m > 0)
                        .map(|m| format!("faltam {m} meses"))
                        .unwrap_or_default(),
                });
            }
        }
        for observacao in &avaliacao.observacoes {
            avisos.push(observacao.clone());
        }
        if avaliacao.regra_escolhida.is_none() {
            avisos.push(
                "Requisitos da espécie não cumpridos na DER. O valor calculado é uma simulação condicional, não uma concessão."
                    .into(),
            );
        }
    }

    let coeficiente_str = format!("{coeficiente}");
    Ok(RgpsResultado {
        segurado: SeguradoView {
            nome: entrada.nome.clone(),
            sexo: entrada.sexo.clone(),
            data_nascimento: entrada.data_nascimento.map(|d| d.to_string()),
        },
        especie: EspecieView {
            codigo: if especie == Especie::TempoContribuicao {
                "42".into()
            } else {
                avaliacao.especie.to_string()
            },
            nome: if especie == Especie::TempoContribuicao {
                "Aposentadoria por tempo de contribuição".into()
            } else {
                avaliacao.nome.to_string()
            },
            regra_escolhida: if especie == Especie::TempoContribuicao {
                transicoes
                    .as_ref()
                    .and_then(|t| t.get("regra_escolhida"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            } else {
                avaliacao.regra_escolhida.map(str::to_string)
            },
            regra_nome: if especie == Especie::TempoContribuicao {
                None
            } else {
                avaliacao
                    .regras
                    .iter()
                    .find(|r| Some(r.id) == avaliacao.regra_escolhida)
                    .map(|r| r.nome.clone())
            },
            elegivel: if especie == Especie::TempoContribuicao {
                transicoes
                    .as_ref()
                    .and_then(|t| t.get("regra_escolhida"))
                    .is_some()
            } else {
                avaliacao.regra_escolhida.is_some()
            },
        },
        primeira_competencia: primeira.to_string(),
        ultima_competencia: ultima.to_string(),
        parametros: ParametrosView {
            divisor,
            divisor_minimo_aplicado: entrada.opcoes.divisor_minimo
                && (quantidade as u32) < crate::DIVISOR_MINIMO,
            anos,
            meses_restantes,
            anos_base: base,
            coeficiente: coeficiente_str.clone(),
            correcao_ativa,
            referencia_correcao: correcao
                .as_ref()
                .map(|pbc| pbc.referencia.clone()),
            descarte_aplicado: correcao.as_ref().map(|pbc| pbc.descarte_aplicado),
        },
        resumo: ResumoView {
            competencias: quantidade,
            carencia_meses: carencia.carencia_meses,
            soma_salarios: match &correcao {
                Some(pbc) => pbc.soma_considerada.clone(),
                None => money(soma_total),
            },
            salario_beneficio: money(salario_beneficio),
            coeficiente: coeficiente_str,
            rmi_bruta: money(rmi_bruta),
            rmi: money(rmi_bruta),
            limite_aplicado,
            teto: teto_derivado.map(money),
            piso: entrada.opcoes.piso.map(money),
        },
        tempo: tempo.map(|t| serde_json::to_value(&t).expect("serialização de tempo")),
        carencia: serde_json::to_value(&carencia).expect("serialização de carência"),
        correcao: correcao
            .as_ref()
            .map(|pbc| serde_json::to_value(pbc).expect("serialização de PBC")),
        memoria,
        transicoes,
        beneficio: if especie == Especie::TempoContribuicao {
            None
        } else {
            Some(serde_json::to_value(&avaliacao).expect("serialização do benefício"))
        },
        avisos,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::parse_money;

    fn competencia(v: &str) -> Competencia {
        Competencia::parse(v, "t").unwrap()
    }

    fn entrada_feminina_300() -> RgpsEntrada {
        RgpsEntrada {
            nome: Some("Cliente Teste".into()),
            especie: None,
            sexo: "feminino".into(),
            data_nascimento: None,
            data_referencia: None,
            vinculos: vec![],
            competencias: (0..300)
                .map(|i| {
                    let total = 1994 * 12 + 6 + i as u16;
                    (
                        Competencia {
                            ano: total / 12,
                            mes: (total % 12 + 1) as u8,
                        },
                        parse_money("2000.00", "s").unwrap(),
                    )
                })
                .collect(),
            opcoes: RgpsOpcoes::default(),
        }
    }

    #[test]
    fn especie_sem_motor_bloqueia() {
        let mut entrada = entrada_feminina_300();
        entrada.especie = Some("80".into());
        let erro = calcular(&entrada).unwrap_err();
        assert!(erro.0.contains("não tem fórmula"), "{erro}");
        entrada.especie = Some("42".into());
        assert!(calcular(&entrada).is_ok());
    }

    #[test]
    fn descarte_ec103_remove_os_menores_ate_o_minimo() {
        let mut entrada = entrada_feminina_300();
        entrada.competencias = (0..10)
            .map(|i| {
                (
                    Competencia { ano: 2024, mes: (i + 1) as u8 },
                    parse_money(&format!("{}.00", (i + 1) * 100), "s").unwrap(),
                )
            })
            .collect();
        entrada.opcoes.corrigir_salarios = true;
        entrada.opcoes.referencia_correcao = Some(Competencia { ano: 2024, mes: 10 });
        entrada.opcoes.descartar_menores_minimo = Some(8);
        let resultado = calcular(&entrada).unwrap();
        let correcao = resultado.correcao.as_ref().unwrap();
        assert_eq!(correcao["pbc"], 10);
        assert_eq!(correcao["consideradas"], 8);
        assert_eq!(correcao["descartadas"], 2);
        assert_eq!(correcao["descarte_aplicado"], true);
        assert_eq!(correcao["parcelas_descartadas"], serde_json::json!(["2024-01", "2024-02"]));
    }
    #[test]
    fn reform_split_usa_tempo_apurado_e_nao_competencias() {
        let mut entrada = entrada_feminina_300();
        entrada.data_nascimento = Some(IsoDate::parse("1960-01-01", "nascimento").unwrap());
        entrada.data_referencia = Some(IsoDate::parse("2025-01-31", "referencia").unwrap());
        entrada.vinculos = vec![VinculoEntrada {
            inicio: IsoDate::parse("2019-01-01", "inicio").unwrap(),
            fim: Some(IsoDate::parse("2019-11-13", "fim").unwrap()),
            tipo: "empregado".into(),
            empresa: None,
            fator: 1.0,
        }];
        let resultado = calcular(&entrada).unwrap();
        let transicoes = resultado.transicoes.unwrap();
        // 2019-01-01..2019-11-13 = 317 dias inclusivos; 317/30 = 10 meses.
        assert_eq!(transicoes["tempo_ate_reforma_meses"], 10);
        assert_eq!(transicoes["tempo_total_meses"], 10);
    }

    #[test]
    fn caso_ancora_mulher_300_competencias() {
        let resultado = calcular(&entrada_feminina_300()).unwrap();
        assert_eq!(resultado.resumo.salario_beneficio, "2000.00");
        assert_eq!(resultado.resumo.coeficiente, "80");
        assert_eq!(resultado.resumo.rmi, "1600.00");
        assert_eq!(resultado.resumo.carencia_meses, 300);
    }

    #[test]
    fn rejeita_duplicada_e_pre_1994_07() {
        let mut entrada = entrada_feminina_300();
        entrada.competencias.truncate(2);
        entrada.competencias[1].0 = entrada.competencias[0].0;
        assert!(calcular(&entrada).is_err());

        let mut entrada = entrada_feminina_300();
        entrada.competencias.clear();
        entrada.competencias.push((competencia("1994-06"), parse_money("100", "s").unwrap()));
        assert!(calcular(&entrada).is_err());
    }

    #[test]
    fn divisor_minimo_50_competencias() {
        let mut entrada = entrada_feminina_300();
        entrada.competencias.truncate(50);
        entrada.opcoes.divisor_minimo = true;
        let resultado = calcular(&entrada).unwrap();
        // 50 × 2000 = 100.000 / 108 = 925.9259... → 925.93; coeficiente 60%; RMI 555.56 (925.93×0.6 = 555.5555...)
        assert_eq!(resultado.resumo.salario_beneficio, "925.93");
        assert_eq!(resultado.resumo.coeficiente, "60");
        assert_eq!(resultado.resumo.rmi, "555.56");
    }

    #[test]
    fn aplica_piso() {
        let mut entrada = entrada_feminina_300();
        entrada.competencias.truncate(12);
        entrada.opcoes.piso = Some(parse_money("1518.00", "p").unwrap());
        let resultado = calcular(&entrada).unwrap();
        // SB 2000 × 60% = 1200 < 1518 → piso.
        assert_eq!(resultado.resumo.limite_aplicado, Some("piso"));
        assert_eq!(resultado.resumo.rmi, "1518.00");
    }
}
