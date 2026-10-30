use crate::{CalculationOutcome, DomainError, Money, Warning};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BpcPerson {
    pub nome: String,
    /// Monthly income in cents. Negative values are refused.
    pub renda_centavos: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BpcIncomeInput {
    /// Minimum wage used for the one-quarter comparison, in cents.
    pub salario_minimo_centavos: i64,
    pub pessoas: Vec<BpcPerson>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BpcIncomeOutput {
    pub total_centavos: i64,
    pub pessoas: u32,
    pub renda_per_capita_centavos: i64,
    pub limite_um_quarto_centavos: i64,
    pub dentro_do_limite_aritmetico: bool,
    pub composicao_confirmada: bool,
}

pub fn calculate_bpc_income(
    input: &BpcIncomeInput,
) -> Result<CalculationOutcome<BpcIncomeOutput>, DomainError> {
    if input.salario_minimo_centavos <= 0 {
        return Err(DomainError::InvalidMoney);
    }
    if input.pessoas.is_empty() || input.pessoas.len() > 100 {
        return Err(DomainError::InvalidInput);
    }
    let mut total = Money::ZERO;
    for person in &input.pessoas {
        if person.nome.trim().is_empty() || person.nome.chars().count() > 160 {
            return Err(DomainError::InvalidInput);
        }
        let income = Money::from_cents(person.renda_centavos).ensure_nonnegative()?;
        total = total.checked_add(income)?;
    }
    let count = i64::try_from(input.pessoas.len()).map_err(|_| DomainError::Overflow)?;
    let per_capita = total.cents() / count;
    let limit = (i128::from(input.salario_minimo_centavos) / 4)
        .try_into()
        .map_err(|_| DomainError::Overflow)?;
    let mut outcome = CalculationOutcome::experimental(
        BpcIncomeOutput {
            total_centavos: total.cents(),
            pessoas: input.pessoas.len() as u32,
            renda_per_capita_centavos: per_capita,
            limite_um_quarto_centavos: limit,
            dentro_do_limite_aritmetico: per_capita <= limit,
            composicao_confirmada: false,
        },
        "bpc_income_v1",
        &[],
        "Soma e divisão declaradas; composição do grupo familiar, exclusões de renda e enquadramento legal não foram decididos.",
    );
    outcome.warnings.extend([
        Warning::new(
            "declared_composition",
            "A composição do grupo familiar e todas as rendas foram declaradas pelo operador; nenhuma pessoa ou renda foi inferida.",
        ),
        Warning::new(
            "income_exclusions_required",
            "Quais rendas entram ou são excluídas do cálculo do BPC depende de regra jurídica e conferência documental.",
        ),
        Warning::new(
            "integer_division",
            "A renda per capita foi truncada em centavos; o resultado é aritmético e não uma concessão.",
        ),
    ]);
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutcomeStatus;

    #[test]
    fn per_capita_is_exact_and_never_negative() {
        let out = calculate_bpc_income(&BpcIncomeInput {
            salario_minimo_centavos: 141_200,
            pessoas: vec![
                BpcPerson { nome: "A".into(), renda_centavos: 50_000 },
                BpcPerson { nome: "B".into(), renda_centavos: 0 },
            ],
        })
        .unwrap();
        let data = out.data.unwrap();
        assert_eq!(data.total_centavos, 50_000);
        assert_eq!(data.renda_per_capita_centavos, 25_000);
        assert_eq!(data.limite_um_quarto_centavos, 35_300);
        assert!(data.dentro_do_limite_aritmetico);
        assert!(!data.composicao_confirmada);
    }

    #[test]
    fn negative_income_is_refused() {
        let out = calculate_bpc_income(&BpcIncomeInput {
            salario_minimo_centavos: 141_200,
            pessoas: vec![BpcPerson { nome: "A".into(), renda_centavos: -1 }],
        });
        assert!(out.is_err());
    }

    #[test]
    fn empty_group_is_refused_without_zero() {
        let out = calculate_bpc_income(&BpcIncomeInput {
            salario_minimo_centavos: 141_200,
            pessoas: vec![],
        });
        assert!(out.is_err());
        let _ = OutcomeStatus::Unsupported;
    }
}

