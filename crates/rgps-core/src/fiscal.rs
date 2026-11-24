//! IRRF mensal a partir da tabela oficial embutida por competência.
//! Não calcula 13º, RRA, pensão alimentícia nem valida enquadramento legal.
use crate::dados::irrf::{IRRF, TabelaIrrf};
use rust_decimal::{Decimal, RoundingStrategy};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct IrrfMensal {
    pub tabela_competencia: &'static str,
    pub base_bruta: Decimal,
    pub deducao_dependentes: Decimal,
    pub isencao_65: Decimal,
    pub base_ajustada: Decimal,
    pub aliquota: Decimal,
    pub deducao: Decimal,
    pub irrf: Decimal,
    /// Redutor mensal da Lei 9.250/1995, art. 3º-A (Lei 15.270/2025), vigente a partir de 01/2026.
    pub redutor_lei_15270: Decimal,
    /// IRRF líquido após o redutor da Lei 15.270/2025 (igual a `irrf` para competências < 2026-01).
    pub irrf_liquido_lei_15270: Decimal,
}

/// Redução mensal da Lei nº 9.250/1995, art. 3º-A (Lei nº 15.270/2025),
/// vigente a partir da competência `2026-01`, limitada ao imposto devido.
pub fn reducao_lei_15270(imposto: Decimal, rend_tributavel: Decimal, competencia: &str) -> Decimal {
    if competencia < "2026-01" || imposto <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let lim_isencao = Decimal::from(5000);
    let lim_teto = Decimal::from(7350);
    if rend_tributavel <= lim_isencao {
        return imposto.min(Decimal::new(31289, 2));
    }
    if rend_tributavel >= lim_teto {
        return Decimal::ZERO;
    }
    let bruto = Decimal::new(97862, 2) - Decimal::new(133145, 6) * rend_tributavel;
    imposto.min(money(bruto.max(Decimal::ZERO)))
}

fn decimal(value: &str, label: &str) -> Result<Decimal, String> {
    Decimal::from_str(value.trim())
        .map_err(|_| format!("{label} inválido; use ponto decimal, sem separador de milhar."))
}

fn money(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// `competencia` no formato AAAA-MM. Usa a tabela mais recente publicada até a
/// competência; competência anterior a 07/1994 é recusada (moeda histórica).
pub fn irrf_mensal(
    competencia: &str,
    base: &str,
    dependentes: u32,
    isencao_65: Option<&str>,
) -> Result<IrrfMensal, String> {
    if competencia.len() != 7 || competencia.as_bytes()[4] != b'-' {
        return Err("Competência do IRRF deve usar AAAA-MM.".into());
    }
    let mes: u32 = competencia[5..7].parse().map_err(|_| "Competência do IRRF inválida.")?;
    if !(1..=12).contains(&mes) {
        return Err("Competência do IRRF inválida.".into());
    }
    if competencia < "1994-07" {
        return Err("IRRF só é calculado a partir de 07/1994; moedas anteriores não são convertidas.".into());
    }
    if dependentes > 20 {
        return Err("Número de dependentes acima do limite suportado (20).".into());
    }
    let base = decimal(base, "Base do IRRF")?;
    if base < Decimal::ZERO {
        return Err("Base do IRRF não pode ser negativa.".into());
    }
    let isencao_65 = match isencao_65 {
        Some(value) if !value.trim().is_empty() => {
            let value = decimal(value, "Isenção de 65 anos")?;
            if value < Decimal::ZERO {
                return Err("Isenção de 65 anos não pode ser negativa.".into());
            }
            value
        }
        _ => Decimal::ZERO,
    };
    let tabela: &TabelaIrrf = IRRF
        .iter()
        .rev()
        .find(|tabela| tabela.competencia <= competencia)
        .ok_or("Competência anterior à primeira tabela de IRRF embutida.")?;
    let deducao_dependentes = Decimal::from(dependentes) * decimal(tabela.dependente, "Dependente")?;
    let base_ajustada = (base - deducao_dependentes - isencao_65).max(Decimal::ZERO);
    let mut faixa = &tabela.faixas[4];
    for candidata in &tabela.faixas {
        if candidata.limite == "0" {
            continue;
        }
        if base_ajustada <= decimal(candidata.limite, "Limite")? {
            faixa = candidata;
            break;
        }
    }
    let aliquota = decimal(faixa.aliquota, "Alíquota")?;
    let deducao = decimal(faixa.deducao, "Dedução")?;
    let bruto = base_ajustada * aliquota / Decimal::from(100) - deducao;
    let irrf = money(bruto.max(Decimal::ZERO));
    let rend_tributavel = (base - isencao_65).max(Decimal::ZERO);
    let redutor_lei_15270 = reducao_lei_15270(irrf, rend_tributavel, competencia);
    let irrf_liquido_lei_15270 = money((irrf - redutor_lei_15270).max(Decimal::ZERO));
    Ok(IrrfMensal {
        tabela_competencia: tabela.competencia,
        base_bruta: base,
        deducao_dependentes: money(deducao_dependentes),
        isencao_65: money(isencao_65),
        base_ajustada: money(base_ajustada),
        aliquota,
        deducao,
        irrf,
        redutor_lei_15270,
        irrf_liquido_lei_15270,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redutor_lei_15270_aplica_faixas_2026() {
        assert_eq!(reducao_lei_15270(Decimal::from(400), Decimal::from(5000), "2026-01"), Decimal::new(31289, 2));
        assert_eq!(reducao_lei_15270(Decimal::from(100), Decimal::from(3000), "2026-01"), Decimal::from(100));
        assert_eq!(reducao_lei_15270(Decimal::from(800), Decimal::from(6000), "2026-01"), Decimal::new(17975, 2));
        assert_eq!(reducao_lei_15270(Decimal::from(800), Decimal::from(7350), "2026-01"), Decimal::ZERO);
        assert_eq!(reducao_lei_15270(Decimal::from(800), Decimal::from(5000), "2025-12"), Decimal::ZERO);
    }
}
