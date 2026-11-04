use crate::atualizacao::{corrigir_salario, fontes_atualizacao, ParcelaCorrigida};
use crate::calendar::Competencia;
use crate::money::{add, div, fixed, money, to_cents, Result};
use rust_decimal::Decimal;
use serde::Serialize;

const DIVISOR_MINIMO: u32 = 108;

#[derive(Debug, Clone, Serialize)]
pub struct ParcelaPbc {
    pub competencia: String,
    pub salario: String,
    pub teto: String,
    pub considerado: String,
    pub indice: String,
    pub corrigido: String,
    pub fonte: String,
    /// Séries de índice efetivamente usadas na correção desta parcela.
    pub series_usadas: Vec<String>,
    /// Perfil de correção versionado aplicado (ausente no caminho de INPC único).
    pub perfil: Option<String>,
    /// Houve fallback documentado pelo INPC na correção desta parcela.
    pub fallback_inpc: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PbcResultado {
    pub referencia: String,
    pub pbc: usize,
    pub consideradas: usize,
    pub descartadas: usize,
    pub parcelas: Vec<ParcelaPbc>,
    pub parcelas_descartadas: Vec<String>,
    pub cenarios: Vec<CenarioPbc>,
    pub soma_considerada: String,
    pub media: String,
    pub salario_beneficio: String,
    pub descarte_aplicado: bool,
    pub fontes_tetos: &'static str,
    pub fontes_inpc: &'static str,
    /// Perfil de correção versionado usado (ex.: "trf3-v1@1"); None no INPC único.
    pub perfil: Option<String>,
    /// Séries de índice efetivamente usadas, sem repetição.
    pub series_utilizadas: Vec<String>,
    /// Substituições documentadas IPC-R→INPC (fallback), mês a mês.
    pub substituicoes_inpc: Vec<String>,
    /// Séries exigidas pelo perfil e ausentes do catálogo no momento do cálculo.
    pub series_ausentes: Vec<String>,
}

/// Comparação descritiva de médias, sem escolher regra nem recalcular tempo.
/// Não representa, por si só, uma RMI: coeficiente, fator e requisitos ficam fora.
#[derive(Debug, Clone, Serialize)]
pub struct CenarioPbc {
    pub rotulo: &'static str,
    pub consideradas: usize,
    pub descartadas: usize,
    pub media: String,
}

fn cenarios_de_media(parcelas: &[ParcelaCorrigida]) -> Result<Vec<CenarioPbc>> {
    let pbc = parcelas.len();
    let corte10 = pbc * 10 / 100;
    let corte20 = pbc * 20 / 100;
    let mut cortes = vec![0usize, corte10, corte20];
    cortes.sort_unstable();
    cortes.dedup();
    let mut ordenadas = parcelas.to_vec();
    ordenadas.sort_by(|a, b| {
        a.corrigido_full
            .partial_cmp(&b.corrigido_full)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut cenarios = Vec::new();
    for corte in cortes {
        if corte >= pbc {
            continue;
        }
        let usadas = &ordenadas[corte..];
        let soma = usadas
            .iter()
            .fold(Decimal::ZERO, |acc, parcela| add(acc, to_cents(parcela.corrigido_full)));
        let media = div(soma, Decimal::from(usadas.len() as i64))?;
        cenarios.push(CenarioPbc {
            rotulo: match corte {
                0 => "Sem corte",
                _ if corte == corte10 => "Corte de 10%",
                _ => "Corte de 20%",
            },
            consideradas: usadas.len(),
            descartadas: corte,
            media: money(to_cents(media)),
        });
    }
    Ok(cenarios)
}

fn publicar_parcelas(parcelas: &[ParcelaCorrigida]) -> Vec<ParcelaPbc> {
    let (tetos, inpc) = fontes_atualizacao();
    parcelas
        .iter()
        .map(|p| ParcelaPbc {
            competencia: p.competencia.to_string(),
            salario: money(p.salario_nominal),
            teto: money(p.teto),
            considerado: money(p.considerado),
            indice: fixed(p.indice, 6),
            corrigido: money(p.corrigido),
            fonte: format!("{tetos} | {inpc}"),
            series_usadas: p.series_usadas.clone(),
            perfil: p.perfil.clone(),
            fallback_inpc: p.fallback_inpc,
        })
        .collect()
}

/// Consolida perfil, séries usadas e competências com fallback a partir das parcelas.
fn consolidar_series(
    parcelas: &[ParcelaCorrigida],
) -> (Option<String>, Vec<String>, Vec<String>) {
    let mut perfil: Option<String> = None;
    let mut series: Vec<String> = Vec::new();
    let mut substituicoes: Vec<String> = Vec::new();
    for parcela in parcelas {
        if perfil.is_none() {
            perfil = parcela.perfil.clone();
        }
        for serie in &parcela.series_usadas {
            if !series.contains(serie) {
                series.push(serie.clone());
            }
        }
        if parcela.fallback_inpc {
            substituicoes.push(parcela.competencia.to_string());
        }
    }
    (perfil, series, substituicoes)
}

/// Corpo comum do PBC corrigido: ordena desc pelo valor cheio (não arredondado),
/// aplica o descarte fixo de 20% quando pedido e fecha a média das consideradas.
fn pbc_de_parcelas(
    parcelas: Vec<ParcelaCorrigida>,
    referencia: &Competencia,
    descarte: bool,
) -> Result<PbcResultado> {
    let pbc = parcelas.len();
    let consideradas = if descarte {
        ((pbc as f64) * 0.8).floor().max(1.0) as usize
    } else {
        pbc
    };

    // Ordena desc pelo valor cheio (não arredondado), como na implementação de referência,
    // sem alterar a ordem cronológica das parcelas publicadas.
    let mut ordenadas: Vec<_> = parcelas.clone();
    ordenadas.sort_by(|a, b| {
        b.corrigido_full
            .partial_cmp(&a.corrigido_full)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let usadas = &ordenadas[..consideradas];
    let descartadas = &ordenadas[consideradas.min(ordenadas.len())..];

    let mut soma = Decimal::ZERO;
    for parcela in usadas {
        soma = add(soma, to_cents(parcela.corrigido_full));
    }
    let media = div(soma, Decimal::from(consideradas as i64))?;
    let salario_beneficio = to_cents(media);

    let (perfil, series_utilizadas, substituicoes_inpc) = consolidar_series(&parcelas);
    Ok(PbcResultado {
        referencia: referencia.to_string(),
        pbc,
        consideradas,
        descartadas: descartadas.len(),
        parcelas: publicar_parcelas(&parcelas),
        parcelas_descartadas: descartadas.iter().map(|p| p.competencia.to_string()).collect(),
        cenarios: cenarios_de_media(&parcelas)?,
        soma_considerada: money(soma),
        media: money(salario_beneficio),
        salario_beneficio: money(salario_beneficio),
        descarte_aplicado: descarte,
        fontes_tetos: fontes_atualizacao().0,
        fontes_inpc: fontes_atualizacao().1,
        perfil,
        series_utilizadas,
        substituicoes_inpc,
        series_ausentes: Vec::new(),
    })
}

/// Porta de calcularRmiComCorrecao: teto da época antes, INPC acumulado,
/// descarte dos 20% menores e média das consideradas.
pub fn calcular_pbc_com_correcao(
    competencias: &[(Competencia, Decimal)],
    referencia: &Competencia,
    descarte: bool,
) -> Result<PbcResultado> {
    let mut parcelas: Vec<_> = Vec::with_capacity(competencias.len());
    for (competencia, salario) in competencias {
        parcelas.push(corrigir_salario(competencia, *salario, referencia)?);
    }
    pbc_de_parcelas(parcelas, referencia, descarte)
}

/// PBC corrigido pelo perfil oficial versionado: teto da época antes, fator por
/// faixa de série do perfil e média das consideradas. Sem a série oficial
/// carregada, o cálculo falha de forma explícita (ou usa o INPC como fallback
/// documentado, quando a política for [`crate::perfil_correcao::PoliticaFallback::FallbackInpcDocumentado`]).
pub fn calcular_pbc_com_perfil(
    competencias: &[(Competencia, Decimal)],
    referencia: &Competencia,
    descarte: bool,
    catalogo: &crate::perfil_correcao::CatalogoSeries,
    perfil: &crate::perfil_correcao::PerfilCorrecao,
    politica: crate::perfil_correcao::PoliticaFallback,
) -> Result<PbcResultado> {
    let mut parcelas: Vec<_> = Vec::with_capacity(competencias.len());
    let mut substituicoes_inpc: Vec<String> = Vec::new();
    let mut series_ausentes: Vec<String> = Vec::new();
    for (competencia, salario) in competencias {
        let fator =
            crate::perfil_correcao::fator_com_perfil(catalogo, perfil, politica, competencia, referencia)?;
        for substituicao in &fator.substituicoes_inpc {
            if !substituicoes_inpc.contains(substituicao) {
                substituicoes_inpc.push(substituicao.clone());
            }
        }
        for serie in &fator.series_ausentes {
            if !series_ausentes.contains(serie) {
                series_ausentes.push(serie.clone());
            }
        }
        parcelas.push(crate::atualizacao::corrigir_salario_com_fator(
            competencia,
            *salario,
            &fator,
        )?);
    }
    let mut resultado = pbc_de_parcelas(parcelas, referencia, descarte)?;
    resultado.substituicoes_inpc = substituicoes_inpc;
    resultado.series_ausentes = series_ausentes;
    Ok(resultado)
}

fn pbc_de_parcelas_minimo(
    parcelas: Vec<ParcelaCorrigida>,
    referencia: &Competencia,
    minimo_competencias: u32,
) -> Result<PbcResultado> {
    if parcelas.is_empty() || minimo_competencias == 0 {
        return crate::money::erro("Descarte exige competências e mínimo maior que zero.");
    }
    let pbc = parcelas.len();
    let minimo = (minimo_competencias as usize).min(pbc);
    let mut ordenadas = parcelas.clone();
    ordenadas.sort_by(|a, b| {
        a.corrigido_full
            .partial_cmp(&b.corrigido_full)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let corte = pbc - minimo;
    let descartadas = &ordenadas[..corte];
    let usadas = &ordenadas[corte..];
    let mut soma = Decimal::ZERO;
    for parcela in usadas {
        soma = add(soma, to_cents(parcela.corrigido_full));
    }
    let media = if minimo == 0 {
        Decimal::ZERO
    } else {
        div(soma, Decimal::from(minimo as i64))?
    };
    let salario_beneficio = to_cents(media);
    let (perfil, series_utilizadas, substituicoes_inpc) = consolidar_series(&parcelas);
    Ok(PbcResultado {
        referencia: referencia.to_string(),
        pbc,
        consideradas: minimo,
        descartadas: corte,
        parcelas_descartadas: descartadas.iter().map(|p| p.competencia.to_string()).collect(),
        cenarios: cenarios_de_media(&parcelas)?,
        soma_considerada: money(soma),
        media: money(salario_beneficio),
        salario_beneficio: money(salario_beneficio),
        descarte_aplicado: corte > 0,
        parcelas: publicar_parcelas(&parcelas),
        fontes_tetos: fontes_atualizacao().0,
        fontes_inpc: fontes_atualizacao().1,
        perfil,
        series_utilizadas,
        substituicoes_inpc,
        series_ausentes: Vec::new(),
    })
}

/// Descarte dos menores salários (EC 103/2019, art. 26, § 6º) até o mínimo de
/// competências que o operador declara necessário para manter o direito.
/// Remove as menores remunerações corrigidas; o divisor é o número restante.
pub fn calcular_pbc_com_descarte_minimo(
    competencias: &[(Competencia, Decimal)],
    referencia: &Competencia,
    minimo_competencias: u32,
) -> Result<PbcResultado> {
    let mut parcelas: Vec<_> = Vec::with_capacity(competencias.len());
    for (competencia, salario) in competencias {
        parcelas.push(corrigir_salario(competencia, *salario, referencia)?);
    }
    pbc_de_parcelas_minimo(parcelas, referencia, minimo_competencias)
}

/// Variante do descarte mínimo usando o perfil de correção versionado.
pub fn calcular_pbc_com_perfil_descarte_minimo(
    competencias: &[(Competencia, Decimal)],
    referencia: &Competencia,
    minimo_competencias: u32,
    catalogo: &crate::perfil_correcao::CatalogoSeries,
    perfil: &crate::perfil_correcao::PerfilCorrecao,
    politica: crate::perfil_correcao::PoliticaFallback,
) -> Result<PbcResultado> {
    let mut parcelas: Vec<_> = Vec::with_capacity(competencias.len());
    let mut substituicoes_inpc: Vec<String> = Vec::new();
    let mut series_ausentes: Vec<String> = Vec::new();
    for (competencia, salario) in competencias {
        let fator = crate::perfil_correcao::fator_com_perfil(
            catalogo, perfil, politica, competencia, referencia,
        )?;
        for substituicao in &fator.substituicoes_inpc {
            if !substituicoes_inpc.contains(substituicao) {
                substituicoes_inpc.push(substituicao.clone());
            }
        }
        for serie in &fator.series_ausentes {
            if !series_ausentes.contains(serie) {
                series_ausentes.push(serie.clone());
            }
        }
        parcelas.push(crate::atualizacao::corrigir_salario_com_fator(
            competencia,
            *salario,
            &fator,
        )?);
    }
    let mut resultado = pbc_de_parcelas_minimo(parcelas, referencia, minimo_competencias)?;
    resultado.substituicoes_inpc = substituicoes_inpc;
    resultado.series_ausentes = series_ausentes;
    Ok(resultado)
}
/// Média simples sem correção (salários já corrigidos), com divisor mínimo opcional.
pub fn salario_beneficio_sem_correcao(
    competencias: &[(Competencia, Decimal)],
    divisor_minimo: bool,
) -> Result<Decimal> {
    let quantidade = competencias.len();
    if quantidade == 0 {
        return crate::money::erro("Média exige competências.");
    }
    let mut soma = Decimal::ZERO;
    for (_, salario) in competencias {
        soma = add(soma, *salario);
    }
    let divisor = if divisor_minimo && (quantidade as u32) < DIVISOR_MINIMO {
        DIVISOR_MINIMO
    } else {
        quantidade as u32
    };
    div(soma, Decimal::from(divisor))
}

/// Coeficiente 60% + 2% por ano excedente a 15 (feminino) / 20 (masculino).
pub fn coeficiente_progressivo(sexo: &str, meses: u32) -> Decimal {
    let anos_base = if sexo == "feminino" { 15 } else { 20 };
    let anos = (meses / 12) as i64;
    let excedente = (anos - anos_base).max(0);
    Decimal::from(60) + Decimal::from(2) * Decimal::from(excedente)
}

pub fn divisor_usado(quantidade: usize, divisor_minimo: bool) -> u32 {
    if divisor_minimo && (quantidade as u32) < DIVISOR_MINIMO {
        DIVISOR_MINIMO
    } else {
        quantidade as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::parse_money;

    #[test]
    fn coeficiente_progressivo_portado() {
        // Homem com 35 anos: 60 + 2×15 = 90. Mulher 25 anos: 60+2×10 = 80.
        assert_eq!(coeficiente_progressivo("masculino", 420), Decimal::from(90));
        assert_eq!(coeficiente_progressivo("feminino", 300), Decimal::from(80));
        // Homem com 14 anos: permanece 60.
        assert_eq!(coeficiente_progressivo("masculino", 168), Decimal::from(60));
    }

    #[test]
    fn divisor_minimo_aplica_108() {
        assert_eq!(divisor_usado(50, true), 108);
        assert_eq!(divisor_usado(50, false), 50);
        assert_eq!(divisor_usado(200, true), 200);
    }

    #[test]
    fn parse_sem_correcao_media_simples() {
        let comps: Vec<(Competencia, Decimal)> = (1..=3)
            .map(|m| {
                (
                    Competencia::parse(&format!("2020-0{m}"), "t").unwrap(),
                    parse_money("1000.00", "s").unwrap(),
                )
            })
            .collect();
        let sb = salario_beneficio_sem_correcao(&comps, false).unwrap();
        assert_eq!(money(sb), "1000.00");
    }

    #[test]
    fn cenarios_de_media_nao_sao_rmi_nem_alteram_tempo() {
        let comps: Vec<(Competencia, Decimal)> = (1..=12)
            .map(|m| {
                (
                    Competencia::parse(&format!("2020-{m:02}"), "t").unwrap(),
                    parse_money("1000.00", "s").unwrap(),
                )
            })
            .collect();
        let pbc = calcular_pbc_com_correcao(
            &comps,
            &Competencia::parse("2021-01", "t").unwrap(),
            false,
        )
        .unwrap();
        assert_eq!(pbc.cenarios.len(), 3);
        assert_eq!(pbc.cenarios[0].consideradas, 12);
        assert_eq!(pbc.cenarios[0].descartadas, 0);
        assert!(pbc.cenarios.iter().any(|c| c.descartadas > 0));
        assert!(pbc.cenarios.iter().all(|c| !c.media.is_empty()));
    }

    #[test]
    fn pbc_com_perfil_registra_series_substituicoes_e_nao_declara_oficial_sem_ipcr() {
        let catalogo = crate::perfil_correcao::CatalogoSeries::compilado().unwrap();
        let perfil = crate::perfil_correcao::perfil_trf3_v1();
        // Uma competência na faixa IPC-R e outra na faixa IGP-DI.
        let comps: Vec<(Competencia, Decimal)> = vec![
            (
                Competencia::parse("1995-06", "t").unwrap(),
                parse_money("2000.00", "s").unwrap(),
            ),
            (
                Competencia::parse("1996-04", "t").unwrap(),
                parse_money("2000.00", "s").unwrap(),
            ),
        ];
        let pbc = calcular_pbc_com_perfil(
            &comps,
            &Competencia::parse("1996-05", "t").unwrap(),
            false,
            &catalogo,
            &perfil,
            crate::perfil_correcao::PoliticaFallback::FallbackInpcDocumentado,
        )
        .unwrap();
        assert_eq!(pbc.perfil.as_deref(), Some("trf3-v1@1"));
        assert_eq!(pbc.series_ausentes, vec!["IPC-R".to_string()]);
        assert!(!pbc.substituicoes_inpc.is_empty(), "fallback registrado");
        assert!(pbc.parcelas.iter().all(|p| p.perfil.as_deref() == Some("trf3-v1@1")));
        let serie_igpdi = pbc
            .parcelas
            .iter()
            .find(|p| p.competencia == "1996-04")
            .unwrap();
        assert_eq!(serie_igpdi.series_usadas, vec!["IGP-DI".to_string()]);
        assert!(!serie_igpdi.fallback_inpc);
        // Sem fallback, a mesma entrada falha nomeando a IPC-R.
        let erro = calcular_pbc_com_perfil(
            &comps,
            &Competencia::parse("1996-05", "t").unwrap(),
            false,
            &catalogo,
            &perfil,
            crate::perfil_correcao::PoliticaFallback::ExigirSerieOficial,
        )
        .unwrap_err();
        assert!(erro.0.contains("IPC-R"), "{erro}");
    }
}
