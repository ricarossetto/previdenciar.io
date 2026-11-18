use crate::calendar::Competencia;
use crate::dados::inpc::{INPC_FONTE, INPC_MENSAL};
use crate::dados::salario_minimo::SALARIO_MINIMO;
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

/// Salário mínimo vigente na competência (usa o último valor <= competência).
pub fn salario_minimo_vigente(competencia: &Competencia) -> Result<Decimal> {
    let alvo = competencia.to_string();
    let mut vigente: Option<(&str, &str)> = None;
    for item in SALARIO_MINIMO {
        if item.0 <= alvo.as_str() {
            vigente = Some(item);
        } else {
            break;
        }
    }
    match vigente {
        Some((comp, sm)) => parse_money(sm, &format!("Salário mínimo da competência {comp}")),
        None => Err(RgpsError(format!(
            "Sem salário mínimo cadastrado para {alvo}."
        ))),
    }
}

/// Última competência com variação INPC publicada na série embutida.
pub fn ultimo_inpc_publicado() -> Option<Competencia> {
    INPC_MENSAL
        .last()
        .and_then(|(comp, _)| Competencia::parse(comp, "INPC").ok())
}

/// Fator de atualização INPC da competência até a referência (ambas inclusas),
/// permitindo limitar ao último INPC publicado quando a referência for mais recente.
pub fn fator_atualizacao_parcial(
    competencia: &Competencia,
    referencia: &Competencia,
) -> Result<(Decimal, Option<Competencia>)> {
    if competencia > referencia {
        return Err(RgpsError(format!(
            "Competência {competencia} é posterior à referência {referencia}."
        )));
    }
    let ultimo = ultimo_inpc_publicado();
    let (limite, clamped) = match ultimo {
        Some(ult) if *referencia > ult => (ult, Some(ult)),
        _ => (*referencia, None),
    };
    if *competencia > limite {
        return Ok((Decimal::ONE, clamped));
    }
    let fator = fator_atualizacao(competencia, &limite)?;
    Ok((fator, clamped))
}

/// Fator de atualização INPC da competência até a referência (ambas inclusas),
/// mesma semântica de fatorAtualizacao() da implementação de referência.
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
    /// Séries de índice efetivamente usadas na correção desta parcela.
    pub series_usadas: Vec<String>,
    /// Perfil de correção versionado aplicado (ausente no caminho de INPC único).
    pub perfil: Option<String>,
    /// Houve fallback documentado pelo INPC na correção desta parcela.
    pub fallback_inpc: bool,
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
        series_usadas: vec!["INPC".to_string()],
        perfil: None,
        fallback_inpc: false,
    })
}

/// Versão tolerante a DIB/referência recente posterior ao último INPC publicado:
/// aplica os índices até o último mês disponível e fator 1,0 para meses futuros.
pub fn corrigir_salario_parcial(
    competencia: &Competencia,
    salario_nominal: Decimal,
    referencia: &Competencia,
) -> Result<(ParcelaCorrigida, Option<Competencia>)> {
    if salario_nominal < Decimal::ZERO {
        return Err(RgpsError(
            "Salário de contribuição não pode ser negativo.".into(),
        ));
    }
    let teto = teto_vigente(competencia)?;
    let considerado = min_dec(salario_nominal, teto);
    let (fator, clamped) = fator_atualizacao_parcial(competencia, referencia)?;
    let corrigido = mul(considerado, fator);
    Ok((
        ParcelaCorrigida {
            competencia: *competencia,
            salario_nominal,
            teto,
            considerado,
            indice: fator,
            corrigido: crate::money::to_cents(corrigido),
            corrigido_full: corrigido,
            series_usadas: vec!["INPC".to_string()],
            perfil: None,
            fallback_inpc: false,
        },
        clamped,
    ))
}

/// min(SC, teto da época) antes da correção; correção pelo fator do perfil de
/// correção versionado ([`crate::perfil_correcao::FatorCorrecao`]), que registra
/// a série usada em cada competência do período corrigido.
pub fn corrigir_salario_com_fator(
    competencia: &Competencia,
    salario_nominal: Decimal,
    fator_correcao: &crate::perfil_correcao::FatorCorrecao,
) -> Result<ParcelaCorrigida> {
    if salario_nominal < Decimal::ZERO {
        return Err(RgpsError(
            "Salário de contribuição não pode ser negativo.".into(),
        ));
    }
    let teto = teto_vigente(competencia)?;
    let considerado = min_dec(salario_nominal, teto);
    let corrigido = mul(considerado, fator_correcao.fator);
    Ok(ParcelaCorrigida {
        competencia: *competencia,
        salario_nominal,
        teto,
        considerado,
        indice: fator_correcao.fator,
        corrigido: crate::money::to_cents(corrigido),
        corrigido_full: corrigido,
        series_usadas: fator_correcao.series_usadas.clone(),
        perfil: Some(format!(
            "{}@{}",
            fator_correcao.perfil_id, fator_correcao.perfil_versao
        )),
        fallback_inpc: !fator_correcao.substituicoes_inpc.is_empty(),
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

    #[test]
    fn corrigir_com_fator_do_perfil_registra_serie_e_perfil() {
        let catalogo = crate::perfil_correcao::CatalogoSeries::compilado().unwrap();
        let perfil = crate::perfil_correcao::perfil_trf3_v1();
        // Faixa IGP-DI: série real, sem fallback.
        let fator = crate::perfil_correcao::fator_com_perfil(
            &catalogo,
            &perfil,
            crate::perfil_correcao::PoliticaFallback::ExigirSerieOficial,
            &Competencia::parse("2004-01", "t").unwrap(),
            &Competencia::parse("2004-01", "t").unwrap(),
        )
        .unwrap();
        let parcela = corrigir_salario_com_fator(
            &Competencia::parse("2004-01", "t").unwrap(),
            Decimal::from(5000),
            &fator,
        )
        .unwrap();
        assert_eq!(money(parcela.teto), "2400.00");
        assert_eq!(money(parcela.considerado), "2400.00");
        assert_eq!(parcela.series_usadas, vec!["IGP-DI".to_string()]);
        assert_eq!(parcela.perfil.as_deref(), Some("trf3-v1@1"));
        assert!(!parcela.fallback_inpc);
    }

    #[test]
    fn corrigir_com_fator_marca_fallback_do_inpc() {
        let catalogo = crate::perfil_correcao::CatalogoSeries::compilado().unwrap();
        let perfil = crate::perfil_correcao::perfil_trf3_v1();
        let fator = crate::perfil_correcao::fator_com_perfil(
            &catalogo,
            &perfil,
            crate::perfil_correcao::PoliticaFallback::FallbackInpcDocumentado,
            &Competencia::parse("1995-05", "t").unwrap(),
            &Competencia::parse("1995-05", "t").unwrap(),
        )
        .unwrap();
        let parcela = corrigir_salario_com_fator(
            &Competencia::parse("1995-05", "t").unwrap(),
            Decimal::from(2000),
            &fator,
        )
        .unwrap();
        // Teto de 1995-05 (832.66) corta o salário antes da correção.
        assert_eq!(money(parcela.considerado), "832.66");
        assert!(parcela.fallback_inpc, "substituição IPC-R→INPC registrada");
    }
}
