//! Conferência de resultados locais contra o cálculo oficial.
//!
//! Regras desta conferência (docs/TRF3_VALIDATION.md — "Revisão crítica"):
//! - O salário de benefício é conferido **antes** e independentemente da RMI.
//!   Com SB divergente, a RMI nem é avaliada: ela herda a divergência.
//! - Equivalência **nunca** é declarada por um único caso. Exige no mínimo
//!   [`MINIMO_CASOS_PARIDADE`] casos sintéticos distintos (faixas de índice,
//!   salários não uniformes, meses extremos, teto), todos conferindo SB e RMI
//!   ao centavo, sem tolerância.
//! - Enquanto a série oficial completa não estiver carregada, a divergência
//!   conhecida fica registrada, não escondida atrás de tolerância.

use crate::money::{div, fixed, money, mul, parse_money, Result, RgpsError};
use rust_decimal::Decimal;
use serde::Serialize;

/// Mínimo de casos sintéticos distintos para declarar equivalência. Um único
/// caso nunca prova equivalência; quatro casos cobrem fronteiras de faixa de
/// índice, série salarial não uniforme, meses extremos e teto.
pub const MINIMO_CASOS_PARIDADE: usize = 4;

/// Um caso sintético conferido contra o resultado oficial (valores em reais,
/// no formato monetário padrão: ponto e duas casas).
#[derive(Debug, Clone)]
pub struct ConferenciaCaso {
    pub rotulo: String,
    pub sb_local: String,
    pub rmi_local: String,
    pub sb_oficial: String,
    pub rmi_oficial: String,
}

/// Veredito de um caso. `rmi_confere` é `None` enquanto o SB divergir: a RMI
/// não é avaliada por cima de um salário de benefício divergente.
#[derive(Debug, Clone, Serialize)]
pub struct VereditoCaso {
    pub rotulo: String,
    pub sb_confere: bool,
    pub rmi_confere: Option<bool>,
    pub divergencia_sb_percentual: String,
    pub divergencia_rmi_percentual: Option<String>,
    pub observacao: String,
}

/// Resultado consolidado da conferência de múltiplos casos.
#[derive(Debug, Clone, Serialize)]
pub struct ConferenciaResultado {
    pub casos: Vec<VereditoCaso>,
    pub sb_confere_todos: bool,
    pub rmi_confere_todos: bool,
    pub equivalencia_declarada: bool,
    pub fundamento: String,
}

/// Divergência percentual do valor local contra o oficial:
/// `(oficial - local) / oficial × 100`. Positiva = local abaixo do oficial.
pub fn divergencia_percentual(local: &str, oficial: &str) -> Result<Decimal> {
    let local = parse_money(local, "valor local")?;
    let oficial = parse_money(oficial, "valor oficial")?;
    if oficial.is_zero() {
        return Err(RgpsError(
            "Valor oficial zerado: a divergência percentual não é definida.".into(),
        ));
    }
    Ok(mul(div(sub(oficial, local), oficial)?, Decimal::from(100)))
}

fn sub(a: Decimal, b: Decimal) -> Decimal {
    a - b
}

fn conferem(a: &str, b: &str) -> Result<bool> {
    Ok(money(parse_money(a, "valor local")?) == money(parse_money(b, "valor oficial")?))
}

/// Confere cada caso: primeiro o SB (independentemente), depois a RMI.
/// A equivalência só é declarada com todos os casos conferindo ao centavo e
/// quantidade mínima de casos.
pub fn conferir_casos(casos: &[ConferenciaCaso]) -> Result<ConferenciaResultado> {
    if casos.is_empty() {
        return Err(RgpsError(
            "Conferência exige ao menos um caso sintético.".into(),
        ));
    }
    let mut vereditos = Vec::with_capacity(casos.len());
    for caso in casos {
        let sb_confere = conferem(&caso.sb_local, &caso.sb_oficial)?;
        let divergencia_sb = divergencia_percentual(&caso.sb_local, &caso.sb_oficial)?;
        // SB primeiro: sem SB conferido, a RMI não recebe veredito.
        let (rmi_confere, divergencia_rmi, observacao) = if !sb_confere {
            (
                None,
                None,
                format!(
                    "SB divergente (local {}, oficial {}): a RMI não é avaliada enquanto o salário de benefício não conferir.",
                    caso.sb_local, caso.sb_oficial
                ),
            )
        } else {
            let rmi_confere = conferem(&caso.rmi_local, &caso.rmi_oficial)?;
            let divergencia_rmi = divergencia_percentual(&caso.rmi_local, &caso.rmi_oficial)?;
            (
                Some(rmi_confere),
                Some(fixed(divergencia_rmi, 4)),
                if rmi_confere {
                    "SB conferido antes da RMI; RMI confere ao centavo.".to_string()
                } else {
                    format!(
                        "SB conferido antes da RMI; RMI diverge (local {}, oficial {}).",
                        caso.rmi_local, caso.rmi_oficial
                    )
                },
            )
        };
        vereditos.push(VereditoCaso {
            rotulo: caso.rotulo.clone(),
            sb_confere,
            rmi_confere,
            divergencia_sb_percentual: fixed(divergencia_sb, 4),
            divergencia_rmi_percentual: divergencia_rmi,
            observacao,
        });
    }
    let sb_confere_todos = vereditos.iter().all(|v| v.sb_confere);
    let rmi_confere_todos = vereditos
        .iter()
        .all(|v| v.rmi_confere == Some(true));
    let equivalencia_declarada =
        sb_confere_todos && rmi_confere_todos && casos.len() >= MINIMO_CASOS_PARIDADE;
    let fundamento = if equivalencia_declarada {
        format!(
            "Paridade declarada somente para este conjunto: {} casos distintos com SB conferido antes da RMI e igualdade ao centavo em todos. Um caso isolado nunca declara equivalência, e a paridade não se estende a entradas fora deste conjunto.",
            casos.len()
        )
    } else if casos.len() < MINIMO_CASOS_PARIDADE {
        format!(
            "Equivalência não declarada: {} caso(s) informado(s); são necessários, no mínimo, {} casos sintéticos distintos. Um único caso nunca prova equivalência.",
            casos.len(),
            MINIMO_CASOS_PARIDADE
        )
    } else if !sb_confere_todos {
        "Equivalência não declarada: há caso(s) com salário de benefício divergente; a RMI herda a divergência do SB.".to_string()
    } else {
        "Equivalência não declarada: há caso(s) com RMI divergente mesmo com SB conferido.".to_string()
    };
    Ok(ConferenciaResultado {
        casos: vereditos,
        sb_confere_todos,
        rmi_confere_todos,
        equivalencia_declarada,
        fundamento,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caso(rotulo: &str, sb_local: &str, rmi_local: &str, sb_oficial: &str, rmi_oficial: &str) -> ConferenciaCaso {
        ConferenciaCaso {
            rotulo: rotulo.into(),
            sb_local: sb_local.into(),
            rmi_local: rmi_local.into(),
            sb_oficial: sb_oficial.into(),
            rmi_oficial: rmi_oficial.into(),
        }
    }

    #[test]
    fn sb_divergente_bloqueia_avaliacao_da_rmi() {
        let resultado = conferir_casos(&[caso("c1", "4724.61", "3779.69", "5083.89", "4067.11")]).unwrap();
        let v = &resultado.casos[0];
        assert!(!v.sb_confere);
        assert_eq!(v.rmi_confere, None, "RMI não avaliada com SB divergente");
        assert!(v.divergencia_rmi_percentual.is_none());
        assert!(!resultado.equivalencia_declarada);
    }

    #[test]
    fn sb_confere_mas_rmi_diverge_e_nao_vira_equivalencia() {
        let resultado = conferir_casos(&[caso("c1", "5083.89", "3000.00", "5083.89", "4067.11")]).unwrap();
        let v = &resultado.casos[0];
        assert!(v.sb_confere);
        assert_eq!(v.rmi_confere, Some(false));
        assert!(!resultado.equivalencia_declarada);
    }

    #[test]
    fn um_caso_nunca_declara_equivalencia_mesmo_igual_ao_centavo() {
        let resultado = conferir_casos(&[caso("c1", "5083.89", "4067.11", "5083.89", "4067.11")]).unwrap();
        assert!(resultado.sb_confere_todos);
        assert!(resultado.rmi_confere_todos);
        assert!(
            !resultado.equivalencia_declarada,
            "um único caso nunca declara equivalência"
        );
        assert!(resultado.fundamento.contains("no mínimo"));
    }

    #[test]
    fn quatro_casos_iguais_ao_centavo_declaram_equivalencia() {
        let casos: Vec<_> = (1..=MINIMO_CASOS_PARIDADE)
            .map(|i| caso(&format!("c{i}"), "1000.00", "800.00", "1000.00", "800.00"))
            .collect();
        let resultado = conferir_casos(&casos).unwrap();
        assert!(resultado.equivalencia_declarada, "{}", resultado.fundamento);
    }

    #[test]
    fn divergencia_percentual_do_caso_trf3() {
        let sb = divergencia_percentual("4724.61", "5083.89").unwrap();
        assert!(sb > Decimal::from(7) && sb < Decimal::from(7) + Decimal::ONE / Decimal::from(10),
            "divergência SB esperada ~7,07%: {sb}");
        let rmi = divergencia_percentual("3779.69", "4067.11").unwrap();
        assert!(rmi > Decimal::from(7) && rmi < Decimal::from(7) + Decimal::ONE / Decimal::from(10),
            "divergência RMI esperada ~7,07%: {rmi}");
    }

    #[test]
    fn caso_vazio_falha() {
        assert!(conferir_casos(&[]).is_err());
    }
}
