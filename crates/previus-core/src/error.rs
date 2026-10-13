use std::fmt;

/// No input values are embedded in errors: callers need not log personal data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    InvalidMoney,
    Overflow,
    NegativeAmount,
    InvalidReferenceBase,
    EmptyIncomes,
    InvalidDate,
    InvalidYearMonth,
    ReversedInterval,
    UnsupportedBranch,
}
impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidMoney => "Formato monetário inválido; use no máximo dois decimais.",
            Self::Overflow => "Valor fora do limite exato suportado.",
            Self::NegativeAmount => "Esta operação não admite valores negativos.",
            Self::InvalidReferenceBase => "A base deve ser informada explicitamente e ser positiva.",
            Self::EmptyIncomes => "Informe ao menos uma renda.",
            Self::InvalidDate => "Data civil inválida (anos permitidos: 0001 a 9999).",
            Self::InvalidYearMonth => "Competência inválida (AAAA-MM).",
            Self::ReversedInterval => "A data final deve ser igual ou posterior à inicial.",
            Self::UnsupportedBranch => "Ramo não implementado; regras faltantes não são presumidas.",
        };
        f.write_str(message)
    }
}
impl std::error::Error for DomainError {}
