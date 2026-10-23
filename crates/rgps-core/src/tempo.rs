use crate::calendar::{Competencia, IsoDate};
use crate::money::{Result, RgpsError};
use std::collections::BTreeSet;

pub const TIPOS_VINCULO: [&str; 8] = [
    "empregado",
    "empregado_domestico",
    "contribuinte_individual",
    "avulso",
    "facultativo",
    "rural",
    "especial",
    "servidor",
];

const MARCO_EC20: &str = "1998-12-16";
const MARCO_EC103: &str = "2019-11-13";

#[derive(Debug, Clone)]
pub struct VinculoEntrada {
    pub inicio: IsoDate,
    pub fim: Option<IsoDate>,
    pub tipo: String,
    pub empresa: Option<String>,
    pub fator: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VinculoProcessado {
    pub indice: usize,
    pub tipo: String,
    pub empresa: Option<String>,
    pub inicio: String,
    pub fim: String,
    pub fator: f64,
    pub dias: i64,
    pub dias_efetivos: i64,
    pub concomitante: bool,
    pub sobreposto: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TempoTotal {
    pub anos: i64,
    pub meses: i64,
    pub dias: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Marcos {
    pub ate_ec20: i64,
    pub ate_ec103: i64,
    pub apos_ec103: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TempoResultado {
    pub vinculos: Vec<VinculoProcessado>,
    pub total_dias: i64,
    pub total_dias_efetivos: i64,
    pub total: TempoTotal,
    pub carencia_meses: usize,
    pub marcos: Marcos,
    pub avisos: Vec<String>,
}

fn validar_fator(fator: f64, label: &str) -> Result<f64> {
    if fator <= 0.0 || fator > 3.0 {
        return Err(RgpsError(format!(
            "Fator inválido: {fator}. Informe um número entre 0 e 3 ({label})."
        )));
    }
    Ok(fator)
}

fn dias_inclusivos(inicio: &IsoDate, fim: &IsoDate) -> i64 {
    inicio.days_between(fim) + 1
}

/// Porta de calcularTempoContribuicao: deduplica sobreposição (cursor), dias/diasEfetivos,
/// carência por competências distintas e marcos EC20/EC103.
pub fn calcular_tempo(
    vinculos: &[VinculoEntrada],
    data_referencia: Option<&IsoDate>,
) -> Result<TempoResultado> {
    if vinculos.is_empty() {
        return Err(RgpsError(
            "Informe ao menos um vínculo para calcular o tempo.".into(),
        ));
    }

    #[derive(Clone)]
    struct Normalizado {
        indice: usize,
        tipo: String,
        empresa: Option<String>,
        inicio: IsoDate,
        fim: IsoDate,
        fator: f64,
    }

    let mut normalizados: Vec<Normalizado> = Vec::new();
    for (index, vinculo) in vinculos.iter().enumerate() {
        let fim = match vinculo.fim {
            Some(fim) => fim,
            None => match data_referencia {
                Some(data) => *data,
                None => {
                    return Err(RgpsError(format!(
                        "Vínculo {} sem fim exige uma data de referência.",
                        index + 1
                    )))
                }
            },
        };
        if fim < vinculo.inicio {
            return Err(RgpsError(format!(
                "Vínculo {} termina antes de começar.",
                index + 1
            )));
        }
        let tipo = if vinculo.tipo.is_empty() {
            "empregado".to_string()
        } else {
            vinculo.tipo.clone()
        };
        if !TIPOS_VINCULO.contains(&tipo.as_str()) {
            return Err(RgpsError(format!("Tipo de vínculo desconhecido: {tipo}.")));
        }
        let fator = validar_fator(vinculo.fator, &format!("vínculo {}", index + 1))?;
        normalizados.push(Normalizado {
            indice: index + 1,
            tipo,
            empresa: vinculo.empresa.clone().map(|e| e.chars().take(120).collect()),
            inicio: vinculo.inicio,
            fim,
            fator,
        });
    }
    normalizados.sort_by(|a, b| {
        (a.inicio, a.fim)
            .cmp(&(b.inicio, b.fim))
    });

    let mut processados: Vec<VinculoProcessado> = Vec::new();
    // Segmentos efetivamente somados (sem sobreposição), para os marcos.
    let mut efetivos: Vec<(IsoDate, IsoDate, f64)> = Vec::new();
    let mut cursor: Option<IsoDate> = None;
    let mut total_dias: i64 = 0;
    let mut total_dias_efetivos: i64 = 0;
    let mut competencias: BTreeSet<Competencia> = BTreeSet::new();

    for vinculo in &normalizados {
        let inicio_efetivo = match cursor {
            Some(cursor) if vinculo.inicio <= cursor => cursor.add_days(1),
            _ => vinculo.inicio,
        };
        let concomitante = inicio_efetivo > vinculo.inicio;
        let sobreposto = inicio_efetivo > vinculo.fim;
        let dias = if sobreposto {
            0
        } else {
            dias_inclusivos(&inicio_efetivo, &vinculo.fim)
        };
        let dias_efetivos = (dias as f64 * vinculo.fator).round() as i64;
        total_dias += dias;
        total_dias_efetivos += dias_efetivos;
        if dias > 0 {
            efetivos.push((inicio_efetivo, vinculo.fim, vinculo.fator));
            let mut periodo = Competencia {
                ano: inicio_efetivo.ano as u16,
                mes: inicio_efetivo.mes,
            };
            let ultimo = Competencia {
                ano: vinculo.fim.ano as u16,
                mes: vinculo.fim.mes,
            };
            while periodo <= ultimo {
                competencias.insert(periodo);
                periodo = periodo.next();
            }
            if cursor.is_none() || vinculo.fim > cursor.unwrap() {
                cursor = Some(vinculo.fim);
            }
        }
        processados.push(VinculoProcessado {
            indice: vinculo.indice,
            tipo: vinculo.tipo.clone(),
            empresa: vinculo.empresa.clone(),
            inicio: vinculo.inicio.to_string(),
            fim: vinculo.fim.to_string(),
            fator: vinculo.fator,
            dias,
            dias_efetivos,
            concomitante,
            sobreposto,
        });
    }

    let anos = total_dias_efetivos / 365;
    let meses = (total_dias_efetivos % 365) / 30;
    let dias = total_dias_efetivos - anos * 365 - meses * 30;

    let marco_ec20 = IsoDate::parse(MARCO_EC20, "marco EC20")?;
    let marco_ec103 = IsoDate::parse(MARCO_EC103, "marco EC103")?;
    let ate_ec20 = efetivos
        .iter()
        .map(|(inicio, fim_vinculo, fator)| {
            let fim = if *fim_vinculo < marco_ec20 { *fim_vinculo } else { marco_ec20 };
            if *inicio <= fim {
                (dias_inclusivos(inicio, &fim) as f64 * fator).round() as i64
            } else {
                0
            }
        })
        .sum();
    let ate_ec103 = efetivos
        .iter()
        .map(|(inicio, fim_vinculo, fator)| {
            let fim = if *fim_vinculo < marco_ec103 { *fim_vinculo } else { marco_ec103 };
            if *inicio <= fim {
                (dias_inclusivos(inicio, &fim) as f64 * fator).round() as i64
            } else {
                0
            }
        })
        .sum();
    let apos_ec103 = efetivos
        .iter()
        .map(|(inicio_vinculo, fim_vinculo, fator)| {
            let inicio = if *inicio_vinculo > marco_ec103 {
                *inicio_vinculo
            } else {
                marco_ec103.add_days(1)
            };
            if inicio <= *fim_vinculo {
                (dias_inclusivos(&inicio, fim_vinculo) as f64 * fator).round() as i64
            } else {
                0
            }
        })
        .sum();

    let mut avisos = vec![
        "Tempo informado é declaratório: confira cada vínculo no CNIS e nos documentos antes de usar o resultado.".into(),
        "Tempo especial, rural e PcD exigem análise própria de enquadramento e não são convertidos automaticamente.".into(),
    ];
    let com_concomitancia = processados
        .iter()
        .filter(|v| v.concomitante)
        .count();
    if com_concomitancia > 0 {
        avisos.push(format!(
            "{com_concomitancia} vínculo(s) com período concomitante: apenas a parcela não sobreposta foi somada."
        ));
    }

    Ok(TempoResultado {
        vinculos: processados,
        total_dias,
        total_dias_efetivos,
        total: TempoTotal { anos, meses, dias },
        carencia_meses: competencias.len(),
        marcos: Marcos {
            ate_ec20,
            ate_ec103,
            apos_ec103,
        },
        avisos,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vinculo(inicio: &str, fim: &str) -> VinculoEntrada {
        VinculoEntrada {
            inicio: IsoDate::parse(inicio, "t").unwrap(),
            fim: Some(IsoDate::parse(fim, "t").unwrap()),
            tipo: "empregado".into(),
            empresa: None,
            fator: 1.0,
        }
    }

    #[test]
    fn tempo_deduplica_sobreposicao() {
        let resultado = calcular_tempo(
            &[vinculo("2015-01-01", "2018-12-31"), vinculo("2018-01-01", "2019-12-31")],
            None,
        )
        .unwrap();
        // 4 anos (2015-2018) + 1 ano sem sobreposição (2019) = 6 anos? 2015..2018 = 1461 dias; 2019 = 365.
        assert_eq!(resultado.total_dias, 1461 + 365);
        assert_eq!(resultado.vinculos[1].concomitante, true);
    }

    #[test]
    fn carencia_conta_competencias_distintas() {
        let resultado =
            calcular_tempo(&[vinculo("2015-01-01", "2015-03-31")], None).unwrap();
        assert_eq!(resultado.carencia_meses, 3);
    }
}
