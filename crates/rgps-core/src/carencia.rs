use crate::calendar::{Competencia, IsoDate};
use crate::money::{Result, RgpsError};
use std::collections::BTreeSet;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Periodo {
    pub inicio: String,
    pub fim: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CarenciaResultado {
    pub competencias_salario: usize,
    pub competencias_vinculo: usize,
    pub competencias_graca: usize,
    pub recolhimentos_atraso: usize,
    pub carencia_meses: usize,
    /// Competências da carência que não vieram de salário nem de vínculo (graça/atraso).
    pub competencias_extras: Vec<String>,
    pub avisos: Vec<String>,
}

/// Carência qualificada: união de competências de salários informados, vínculos,
/// períodos de graça (afastamento/benefícios, art. 15 Lei 8.213/1991) e recolhimentos em atraso.
pub fn calcular_carencia(
    competencias_salario: &[Competencia],
    vinculos: &[(IsoDate, IsoDate)],
    graca: &[(IsoDate, IsoDate)],
    recolhimentos_atraso: &[Competencia],
) -> Result<CarenciaResultado> {
    let mut todos: BTreeSet<Competencia> = BTreeSet::new();
    let mut de_salario: BTreeSet<Competencia> = BTreeSet::new();
    let mut de_vinculo: BTreeSet<Competencia> = BTreeSet::new();
    let mut de_graca: BTreeSet<Competencia> = BTreeSet::new();
    let mut de_atraso: BTreeSet<Competencia> = BTreeSet::new();

    for comp in competencias_salario {
        de_salario.insert(*comp);
        todos.insert(*comp);
    }
    for (inicio, fim) in vinculos {
        if fim < inicio {
            return Err(RgpsError(
                "Período de vínculo da carência termina antes de começar.".into(),
            ));
        }
        let mut cursor = Competencia {
            ano: inicio.ano as u16,
            mes: inicio.mes,
        };
        let ultimo = Competencia {
            ano: fim.ano as u16,
            mes: fim.mes,
        };
        while cursor <= ultimo {
            de_vinculo.insert(cursor);
            todos.insert(cursor);
            cursor = cursor.next();
        }
    }
    for (inicio, fim) in graca {
        if fim < inicio {
            return Err(RgpsError(
                "Período de graça termina antes de começar.".into(),
            ));
        }
        let mut cursor = Competencia {
            ano: inicio.ano as u16,
            mes: inicio.mes,
        };
        let ultimo = Competencia {
            ano: fim.ano as u16,
            mes: fim.mes,
        };
        while cursor <= ultimo {
            de_graca.insert(cursor);
            todos.insert(cursor);
            cursor = cursor.next();
        }
    }
    for comp in recolhimentos_atraso {
        de_atraso.insert(*comp);
        todos.insert(*comp);
    }

    let extras: Vec<String> = todos
        .iter()
        .filter(|c| !de_salario.contains(c) && !de_vinculo.contains(c))
        .map(|c| c.to_string())
        .collect();

    let avisos = vec![
        "Carência apurada por competências com contribuição, período de graça e recolhimentos em atraso; confira no CNIS.".into(),
        "Recolhimento em atraso só conta como carência na forma do art. 30, §§ 3º-5º, do Decreto 3.048/1999; a cobrança com juros é pré-requisito.".into(),
    ];

    Ok(CarenciaResultado {
        competencias_salario: de_salario.len(),
        competencias_vinculo: de_vinculo.len(),
        competencias_graca: de_graca
            .iter()
            .filter(|c| !de_salario.contains(c) && !de_vinculo.contains(c))
            .count(),
        recolhimentos_atraso: de_atraso
            .iter()
            .filter(|c| !de_salario.contains(c) && !de_vinculo.contains(c) && !de_graca.contains(c))
            .count(),
        carencia_meses: todos.len(),
        competencias_extras: extras,
        avisos,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(v: &str) -> IsoDate {
        IsoDate::parse(v, "t").unwrap()
    }

    #[test]
    fn uniao_sem_duplicar() {
        let salarios = vec![
            Competencia::parse("2020-01", "t").unwrap(),
            Competencia::parse("2020-02", "t").unwrap(),
        ];
        let vinculos = vec![(data("2020-02-01"), data("2020-04-30"))];
        let graca = vec![(data("2020-05-01"), data("2020-06-30"))];
        let atraso = vec![Competencia::parse("2020-07", "t").unwrap()];
        let resultado = calcular_carencia(&salarios, &vinculos, &graca, &atraso).unwrap();
        assert_eq!(resultado.carencia_meses, 7); // jan..jul/2020
        assert_eq!(resultado.competencias_extras.len(), 3); // mai, jun, jul
    }
}
