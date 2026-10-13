use crate::atualizacao::teto_vigente;
use crate::calendar::{Competencia, IsoDate};
use crate::money::{money, parse_money, Result, RgpsError};
use crate::rmi::{calcular_pbc_com_correcao, salario_beneficio_sem_correcao};
use crate::transicoes::{build_transicoes, TransicoesEntrada};
use rust_decimal::Decimal;
use serde::Serialize;

const LIMITE_SIMULACAO_MESES: u32 = 480;

#[derive(Debug, Clone, Serialize)]
pub struct ProjecaoResultado {
    pub salario_simulacao: String,
    pub meses_simulados: u32,
    pub der: Option<String>,
    pub regra_escolhida: Option<&'static str>,
    pub numero_regra_transicao: Option<u8>,
    pub rmi: Option<String>,
    pub salario_beneficio: Option<String>,
    pub idade_der_meses: Option<i64>,
    pub tempo_der_meses: Option<i64>,
    pub carencia_der: Option<usize>,
    pub regra_alvo: Option<String>,
    pub avisos: Vec<String>,
}

/// Entrada da projeção — construída a partir da RgpsEntrada pelo engine.
pub struct ProjecaoRequisicao {
    pub sexo: String,
    pub data_nascimento: IsoDate,
    pub data_referencia: IsoDate,
    pub competencias: Vec<(Competencia, Decimal)>,
    pub corrigir_salarios: bool,
    pub descarte: bool,
    pub salario_simulacao: Option<Decimal>,
    pub teto: Option<Decimal>,
    pub piso: Option<Decimal>,
    pub aplicar_teto_vigente_der: bool,
    pub expectativa_sobrevida: Option<Decimal>,
    pub regra_alvo: Option<String>,
}

fn meses_ate_reforma(competencias: &[(Competencia, Decimal)]) -> i64 {
    const REFORMA: Competencia = Competencia { ano: 2019, mes: 11 };
    competencias
        .iter()
        .filter(|(c, _)| *c <= REFORMA)
        .count() as i64
}

fn elegibilidade(
    sexo: &str,
    nascimento: &IsoDate,
    der: &IsoDate,
    competencias_len: i64,
    meses_ate_reforma: i64,
) -> Vec<&'static str> {
    let tempo_total = competencias_len;
    let idade = crate::calendar::idade_em_meses(nascimento, der);
    let ano = der.ano;
    let tempo_minimo = if sexo == "feminino" { 360 } else { 420 };
    let mut elegiveis = Vec::new();
    // pontos
    let pontos_exigidos = crate::transicoes::pontos_exigidos(sexo, ano);
    if tempo_total >= tempo_minimo && pontos_exigidos <= (tempo_total / 12) + (idade / 12) {
        elegiveis.push("pontos");
    }
    // idade progressiva
    if tempo_total >= tempo_minimo && idade >= crate::transicoes::idade_exigida_meses(sexo, ano) {
        elegiveis.push("idade-progressiva");
    }
    // pedágios (art. 17 e art. 20): dependem dos meses até a reforma
    let faltava = (tempo_minimo - meses_ate_reforma).max(0);
    let aplica50 = faltava > 0 && faltava <= 24;
    if aplica50 && tempo_total >= tempo_minimo + (faltava + 1) / 2 {
        elegiveis.push("pedagio-50");
    }
    let idade_pedagio100 = if sexo == "feminino" { 57 * 12 } else { 60 * 12 };
    let aplica100 = meses_ate_reforma > 0 && meses_ate_reforma < tempo_minimo;
    if aplica100 && idade >= idade_pedagio100 && tempo_total >= tempo_minimo + faltava {
        elegiveis.push("pedagio-100");
    }
    // idade permanente
    let idade_regra = if sexo == "feminino" { 62 * 12 } else { 65 * 12 };
    if tempo_total >= 180 && idade >= idade_regra {
        elegiveis.push("idade-permanente");
    }
    elegiveis
}

/// Simulação de DER futura: assume contribuições contínuas com salário fixo
/// (último salário informado, por padrão) até a primeira regra elegível.
pub fn projetar(req: &ProjecaoRequisicao) -> Result<ProjecaoResultado> {
    let salario_simulacao = req
        .salario_simulacao
        .unwrap_or(req.competencias.last().map(|(_, s)| *s).ok_or_else(|| {
            RgpsError("Projeção exige ao menos uma competência.".into())
        })?);
    if salario_simulacao <= Decimal::ZERO {
        return Err(RgpsError(
            "Salário de simulação deve ser maior que zero.".into(),
        ));
    }

    let mut avisos = vec![
        "Projeção assume contribuições contínuas com salário fixo (último salário ou o informado); períodos sem contribuição, tempo especial e concomitância não são simulados.".into(),
        "A projeção é estimativa e não constitui certidão de tempo de contribuição.".into(),
    ];

    for m in 0..=LIMITE_SIMULACAO_MESES {
        let comp_der = req
            .competencias
            .last()
            .unwrap()
            .0
            .next()
            .checked_add_months(m)
            .ok_or_else(|| RgpsError("Competência excede o limite representável.".into()))?;
        let der = req.data_referencia.add_months_iso(m as i64);
        let total = req.competencias.len() as i64 + m as i64;

        let elegiveis = elegibilidade(
            &req.sexo,
            &req.data_nascimento,
            &der,
            total,
            meses_ate_reforma(&req.competencias),
        );
        let atende = match &req.regra_alvo {
            Some(alvo) => elegiveis.iter().any(|id| id == alvo),
            None => !elegiveis.is_empty(),
        };
        if !atende {
            continue;
        }

        // PBC no mês da DER.
        let mut competencias = req.competencias.clone();
        let mut cursor = req.competencias.last().unwrap().0.next();
        for _ in 0..m {
            competencias.push((cursor, salario_simulacao));
            cursor = cursor.next();
        }

        let (salario_beneficio, consideradas) = if req.corrigir_salarios {
            let pbc = calcular_pbc_com_correcao(&competencias, &comp_der, req.descarte)?;
            (
                parse_money(&pbc.salario_beneficio, "salário de benefício")?,
                pbc.consideradas,
            )
        } else {
            let sb = salario_beneficio_sem_correcao(&competencias, false)?;
            (sb, competencias.len())
        };

        let teto_der = if req.aplicar_teto_vigente_der {
            Some(teto_vigente(&comp_der)?)
        } else {
            req.teto
        };
        let transicoes = build_transicoes(&TransicoesEntrada {
            sexo: req.sexo.clone(),
            data_nascimento: req.data_nascimento,
            data_referencia: der,
            meses_ate_reforma: meses_ate_reforma(&competencias),
            meses_apos_reforma: total - meses_ate_reforma(&competencias),
            salario_beneficio,
            teto: teto_der,
            piso: req.piso,
            expectativa_sobrevida: req.expectativa_sobrevida,
        })?;

        return Ok(ProjecaoResultado {
            salario_simulacao: money(salario_simulacao),
            meses_simulados: m,
            der: Some(der.to_string()),
            regra_escolhida: transicoes.regra_escolhida,
            numero_regra_transicao: transicoes.numero_regra_transicao,
            rmi: transicoes.rmi_escolhida,
            salario_beneficio: Some(money(salario_beneficio)),
            idade_der_meses: Some(transicoes.idade_meses),
            tempo_der_meses: Some(transicoes.tempo_total_meses),
            carencia_der: Some(consideradas.max(total as usize)),
            regra_alvo: req.regra_alvo.clone(),
            avisos,
        });
    }

    avisos.push(format!(
        "Nenhuma regra elegível em {LIMITE_SIMULACAO_MESES} meses de simulação (40 anos)."
    ));
    Ok(ProjecaoResultado {
        salario_simulacao: money(salario_simulacao),
        meses_simulados: LIMITE_SIMULACAO_MESES,
        der: None,
        regra_escolhida: None,
        numero_regra_transicao: None,
        rmi: None,
        salario_beneficio: None,
        idade_der_meses: None,
        tempo_der_meses: None,
        carencia_der: None,
        regra_alvo: req.regra_alvo.clone(),
        avisos,
    })
}

impl Competencia {
    fn checked_add_months(&self, meses: u32) -> Option<Competencia> {
        let total = self.ano as u32 * 12 + (self.mes as u32 - 1) + meses;
        if total / 12 > 9999 {
            return None;
        }
        Some(Competencia {
            ano: (total / 12) as u16,
            mes: (total % 12 + 1) as u8,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::parse_money;
    use std::collections::BTreeMap;

    fn comp(v: &str) -> Competencia {
        Competencia::parse(v, "t").unwrap()
    }

    #[test]
    fn homen_60_anos_30_tempo_projeta_para_pontos() {
        // Homem nascido 1966-05-10, 360 competências de 1996-06 a 2026-05, DER ref 2026-10-09.
        let mut competencias: BTreeMap<String, Decimal> = BTreeMap::new();
        let mut cursor = comp("1996-06");
        for _ in 0..360 {
            competencias.insert(cursor.to_string(), parse_money("3000.00", "s").unwrap());
            cursor = cursor.next();
        }
        let req = ProjecaoRequisicao {
            sexo: "masculino".into(),
            data_nascimento: IsoDate::parse("1966-05-10", "t").unwrap(),
            data_referencia: IsoDate::parse("2026-10-09", "t").unwrap(),
            competencias: competencias.into_iter().map(|(c, s)| (Competencia::parse(&c, "t").unwrap(), s)).collect(),
            corrigir_salarios: false,
            descarte: false,
            salario_simulacao: None,
            teto: None,
            piso: None,
            aplicar_teto_vigente_der: false,
            expectativa_sobrevida: None,
            regra_alvo: None,
        };
        let resultado = projetar(&req).unwrap();
        assert!(resultado.der.is_some(), "deve projetar DER: {resultado:?}");
        assert!(resultado.meses_simulados <= 480);
        // Idade no DER: 1966-05-10 + simulação; pontos exigidos no ano do DER ≤ 105.
        let idade = resultado.idade_der_meses.unwrap();
        assert!(idade >= 60 * 12);
    }
}
