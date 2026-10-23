use crate::calendar::Competencia;
use crate::dados::inpc::{INPC_FONTE, INPC_MENSAL};
use crate::dados::tetos::{TETOS_CONTRIBUICAO, TETOS_FONTE};
use crate::money::{add, min_dec, mul, parse_money, Result, RgpsError};
use rust_decimal::Decimal;

fn lookup(tabela: &[(&'static str, &'static str)], competencia: &Competencia, label: &str) -> Result<Decimal> {
    let alvo = competencia.to_string();
    for (comp, valor) in tabela {
        match (*comp).cmp(alvo.as_str()) {
            std::cmp::Ordering::Equal => {
                return parse_money(valor, label);
            }
            std::cmp::Ordering::Greater => break,
            std::cmp::Ordering::Less => continue,
        }
    }
    Err(RgpsError(format!("Sem valor cadastrado para {alvo}.")))
}

/// Teto do salário de contribuição vigente na competência (portado de tabelas-atualizacao.mjs).
pub fn teto_vigente(competencia: &Competencia) -> Result<Decimal> {
    let alvo = competencia.to_string();
    let mut vigente: Option<(&str, &str)> = None;
    for item in TETOS_CONTRIBUICAO {
        if item.0 <= alvo.as_str() {
            vigente = Some(item);
        } else {
            break;
        }
    }
    match vigente {
        Some((comp, teto)) => parse_money(teto, &format!("Teto da competência {comp}")),
        None => Err(RgpsError(format!(
            "Sem teto de contribuição cadastrado para {alvo}."
        ))),
    }
}

/// Fator de atualização INPC da competência até a referência (ambas inclusas),
/// mesma semântica de fatorAtualizacao() do ATRIUM.
pub fn fator_atualizacao(
    competencia: &Competencia,
    referencia: &Competencia,
) -> Result<Decimal> {
    if competencia > referencia {
        return Err(RgpsError(format!(
            "Competência {competencia} é posterior à referência {referencia}."
        )));
    }
    let mut fator = Decimal::ONE;
    let mut cursor = *competencia;
    while cursor <= *referencia {
        let taxa = lookup(&INPC_MENSAL, &cursor, "INPC")?;
        fator = mul(fator, add(Decimal::ONE, taxa / Decimal::from(100)));
        cursor = cursor.next();
    }
    Ok(fator)
}

#[derive(Debug, Clone)]
pub struct ParcelaCorrigida {
    pub competencia: Competencia,
    pub salario_nominal: Decimal,
    pub teto: Decimal,
    pub considerado: Decimal,
    pub indice: Decimal,
    pub corrigido: Decimal,
    /// Valor cheio (sem arredondar a 2 casas), usado na ordenação do descarte.
    pub corrigido_full: Decimal,
}

/// min(SC, teto da época) antes da correção; correção pelo INPC acumulado.
pub fn corrigir_salario(
    competencia: &Competencia,
    salario_nominal: Decimal,
    referencia: &Competencia,
) -> Result<ParcelaCorrigida> {
    if salario_nominal < Decimal::ZERO {
        return Err(RgpsError(
            "Salário de contribuição não pode ser negativo.".into(),
        ));
    }
    let teto = teto_vigente(competencia)?;
    let considerado = min_dec(salario_nominal, teto);
    let fator = fator_atualizacao(competencia, referencia)?;
    let corrigido = mul(considerado, fator);
    Ok(ParcelaCorrigida {
        competencia: *competencia,
        salario_nominal,
        teto,
        considerado,
        indice: fator,
        corrigido: crate::money::to_cents(corrigido),
        corrigido_full: corrigido,
    })
}

pub fn fontes_atualizacao() -> (&'static str, &'static str) {
    (TETOS_FONTE, INPC_FONTE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::money;
    use rust_decimal::prelude::FromStr;

    #[test]
    fn teto_da_epoca_2004_01() {
        let comp = Competencia::parse("2004-01", "t").unwrap();
        let teto = teto_vigente(&comp).unwrap();
        assert_eq!(money(teto), "2400.00");
    }

    #[test]
    fn fator_acumula_inclusive_referencia() {
        let comp = Competencia::parse("2004-01", "t").unwrap();
        let refc = Competencia::parse("2004-03", "t").unwrap();
        let fator = fator_atualizacao(&comp, &refc).unwrap();
        // INPC 2004-02 0.39%, 2004-03 0.57% (referência inclusa); competência própria não entra em 2004-01? ENTRA (cursor começa na competência).
        // 0.83 × 1.0039 × 1.0057
        let esperado = Decimal::from_str("1.0083").unwrap()
            * (Decimal::ONE + Decimal::from_str("0.39").unwrap() / Decimal::from(100))
            * (Decimal::ONE + Decimal::from_str("0.57").unwrap() / Decimal::from(100));
        assert_eq!(fator.round_dp(8), esperado.round_dp(8));
    }

    #[test]
    fn corrigir_aplica_teto_antes() {
        let comp = Competencia::parse("2004-01", "t").unwrap();
        let refc = Competencia::parse("2025-02", "t").unwrap();
        let parcela = corrigir_salario(&comp, Decimal::from(5000), &refc).unwrap();
        assert_eq!(money(parcela.teto), "2400.00");
        assert_eq!(money(parcela.considerado), "2400.00");
    }
}
