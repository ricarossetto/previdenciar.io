//! Conferência de resultados locais contra o cálculo oficial.
//!
//! Regras desta conferência (docs/TRF3_VALIDATION.md — "Revisão crítica"):
//! - O salário de benefício é conferido **antes** e independentemente da RMI.
//!   Com SB divergente, a RMI nem é avaliada: ela herda a divergência.
//! - Equivalência **nunca** é declarada por um único caso. Exige no mínimo
//!   [`MINIMO_CASOS_PARIDADE`] casos, cada um com **tupla de valores distinta**
//!   (ver [`ConferenciaResultado::casos_distintos`]), todos conferindo SB e RMI
//!   ao centavo, sem tolerância.
//! - O rótulo do caso **não** entra na prova: quatro cópias dos mesmos valores
//!   com rótulos diferentes continuam sendo um único caso.
//! - [`ConferenciaCaso`] carrega apenas valores conferidos, não as entradas que
//!   os produziram. A conferência não consegue, portanto, atestar cobertura de
//!   fronteiras de faixa de índice, série salarial não uniforme ou teto. A
//!   declaração vale só para o conjunto apresentado, e o fundamento diz isso.
//! - Enquanto a série oficial completa não estiver carregada, a divergência
//!   conhecida fica registrada, não escondida atrás de tolerância.

use crate::money::{div, fixed, money, mul, parse_money, Result, RgpsError};
use rust_decimal::Decimal;
use serde::Serialize;

/// Mínimo de casos com tupla distinta para declarar equivalência. Um único
/// caso nunca prova equivalência. O mínimo conta **tuplas de valores
/// distintas**, não entradas na lista: quatro cópias dos mesmos valores com
/// rótulos diferentes contam como um caso.
pub const MINIMO_CASOS_PARIDADE: usize = 4;

/// Um caso conferido contra o resultado oficial (valores em reais, no formato
/// monetário padrão: ponto e duas casas).
///
/// O `rotulo` é identificação de leitura e **não** entra na prova de
/// diversidade: dois casos com os mesmos quatro valores e rótulos diferentes
/// são tratados como o mesmo caso. A estrutura também não guarda as entradas
/// que geraram os valores (faixas, competências, teto), então a conferência não
/// pode atestar cobertura de fronteiras — ver [`ConferenciaResultado`].
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
///
/// `casos_distintos` é a contagem de tuplas normalizadas distintas (SB local,
/// RMI local, SB oficial, RMI oficial), sem considerar rótulo. É ela que
/// alimenta a exigência de [`MINIMO_CASOS_PARIDADE`]. `equivalencia_declarada`
/// fala apenas deste conjunto: não é selo de cobertura normativa.
#[derive(Debug, Clone, Serialize)]
pub struct ConferenciaResultado {
    pub casos: Vec<VereditoCaso>,
    pub sb_confere_todos: bool,
    pub rmi_confere_todos: bool,
    /// Tuplas de valores distintas (o rótulo não conta).
    pub casos_distintos: usize,
    /// Entradas na lista, incluindo repetições.
    pub casos_informados: usize,
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

/// Tupla normalizada que identifica um caso para a prova de diversidade.
/// Normaliza pelo valor monetário padrão (`money`), de modo que "1000.00" e
/// "1000" são a mesma tupla. O rótulo fica de fora de propósito.
fn tupla_normalizada(caso: &ConferenciaCaso) -> Result<(String, String, String, String)> {
    Ok((
        money(parse_money(&caso.sb_local, "SB local")?),
        money(parse_money(&caso.rmi_local, "RMI local")?),
        money(parse_money(&caso.sb_oficial, "SB oficial")?),
        money(parse_money(&caso.rmi_oficial, "RMI oficial")?),
    ))
}

/// Confere cada caso: primeiro o SB (independentemente), depois a RMI.
/// A equivalência só é declarada com todos os casos conferindo ao centavo e
/// com [`MINIMO_CASOS_PARIDADE`] **tuplas de valores distintas**. Repetir os
/// mesmos valores com outro rótulo não acrescenta caso.
pub fn conferir_casos(casos: &[ConferenciaCaso]) -> Result<ConferenciaResultado> {
    if casos.is_empty() {
        return Err(RgpsError(
            "Conferência exige ao menos um caso sintético.".into(),
        ));
    }
    let mut vereditos = Vec::with_capacity(casos.len());
    let mut distintas: std::collections::BTreeSet<(String, String, String, String)> =
        std::collections::BTreeSet::new();
    for caso in casos {
        let sb_confere = conferem(&caso.sb_local, &caso.sb_oficial)?;
        let divergencia_sb = divergencia_percentual(&caso.sb_local, &caso.sb_oficial)?;
        // Tupla calculada aqui: além de contar diversidade, valida os quatro
        // valores monetários de todo caso, inclusive os de SB divergente.
        let tupla = tupla_normalizada(caso)?;
        let repetida = !distintas.insert(tupla);
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
        if repetida {
            if let Some(ultimo) = vereditos.last_mut() {
                ultimo.observacao.push_str(
                    " Valores repetem outro caso deste conjunto; o rótulo não cria um caso novo.",
                );
            }
        }
    }
    let casos_distintos = distintas.len();
    let casos_informados = casos.len();
    let sb_confere_todos = vereditos.iter().all(|v| v.sb_confere);
    let rmi_confere_todos = vereditos
        .iter()
        .all(|v| v.rmi_confere == Some(true));
    let equivalencia_declarada = sb_confere_todos
        && rmi_confere_todos
        && casos_distintos >= MINIMO_CASOS_PARIDADE;
    let fundamento = if equivalencia_declarada {
        format!(
            "Paridade declarada somente para este conjunto: {casos_distintos} tupla(s) de valores distinta(s) \
             ({casos_informados} caso(s) informado(s)), com SB conferido antes da RMI e igualdade ao centavo em todos. \
             O rótulo não conta como caso novo. Esta conferência compara valores já produzidos e não guarda as entradas \
             que os geraram: ela NÃO comprova cobertura de fronteiras de faixa de índice, série salarial não uniforme, \
             meses extremos ou teto, e a paridade não se estende a entradas fora deste conjunto."
        )
    } else if casos_distintos < MINIMO_CASOS_PARIDADE {
        format!(
            "Equivalência não declarada: {casos_distintos} tupla(s) de valores distinta(s) em {casos_informados} caso(s) \
             informado(s); são necessárias, no mínimo, {MINIMO_CASOS_PARIDADE} tuplas distintas. \
             Rótulos diferentes sobre os mesmos valores contam como um único caso, e um único caso nunca prova equivalência."
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
        casos_distintos,
        casos_informados,
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
    fn quatro_copias_com_rotulos_diferentes_nao_declaram_equivalencia() {
        // Quatro entradas, um único caso: os valores são idênticos e o rótulo
        // não é prova de diversidade.
        let casos: Vec<_> = (1..=MINIMO_CASOS_PARIDADE)
            .map(|i| caso(&format!("c{i}"), "1000.00", "800.00", "1000.00", "800.00"))
            .collect();
        let resultado = conferir_casos(&casos).unwrap();
        assert!(resultado.sb_confere_todos && resultado.rmi_confere_todos);
        assert_eq!(resultado.casos_informados, 4);
        assert_eq!(resultado.casos_distintos, 1, "mesmos valores = um caso");
        assert!(
            !resultado.equivalencia_declarada,
            "quatro cópias não declaram equivalência: {}",
            resultado.fundamento
        );
        assert!(resultado.fundamento.contains("tuplas distintas"), "{}", resultado.fundamento);
        assert!(
            resultado.casos.iter().skip(1).all(|v| v.observacao.contains("rótulo não cria")),
            "a repetição fica visível no veredito"
        );
    }

    #[test]
    fn valor_reescrito_com_outra_escala_conta_como_a_mesma_tupla() {
        // "800" e "800.00" são o mesmo valor normalizado.
        let resultado = conferir_casos(&[
            caso("a", "1000.00", "800.00", "1000.00", "800.00"),
            caso("b", "1000", "800", "1000", "800.00"),
        ])
        .unwrap();
        assert_eq!(resultado.casos_distintos, 1, "a normalização monetária unifica");
    }

    #[test]
    fn quatro_tuplas_distintas_conferindo_declaram_equivalencia_do_conjunto() {
        let pares = [
            ("1000.00", "800.00"),
            ("1200.00", "960.00"),
            ("1400.00", "1120.00"),
            ("1600.00", "1280.00"),
        ];
        let casos: Vec<_> = pares
            .iter()
            .enumerate()
            .map(|(i, (sb, rmi))| caso(&format!("c{i}"), sb, rmi, sb, rmi))
            .collect();
        let resultado = conferir_casos(&casos).unwrap();
        assert_eq!(resultado.casos_distintos, MINIMO_CASOS_PARIDADE);
        assert!(
            resultado.equivalencia_declarada,
            "{}",
            resultado.fundamento
        );
        // A declaração é do conjunto e diz o que não prova.
        assert!(resultado.fundamento.contains("somente para este conjunto"));
        assert!(
            resultado.fundamento.contains("NÃO comprova cobertura"),
            "o fundamento precisa limitar a alegação: {}",
            resultado.fundamento
        );
    }

    #[test]
    fn tres_tuplas_distintas_mais_tres_copias_nao_chegam_ao_minimo() {
        let mut casos: Vec<_> = [
            ("1000.00", "800.00"),
            ("1200.00", "960.00"),
            ("1400.00", "1120.00"),
        ]
        .iter()
        .enumerate()
        .map(|(i, (sb, rmi))| caso(&format!("c{i}"), sb, rmi, sb, rmi))
        .collect();
        // Três repetições com rótulos distintos: não somam casos.
        for i in 0..3 {
            casos.push(caso(&format!("extra{i}"), "1000.00", "800.00", "1000.00", "800.00"));
        }
        let resultado = conferir_casos(&casos).unwrap();
        assert_eq!(resultado.casos_informados, 6);
        assert_eq!(resultado.casos_distintos, 3);
        assert!(!resultado.equivalencia_declarada, "{}", resultado.fundamento);
    }

    #[test]
    fn rotulo_vazio_ou_repetido_nao_altera_a_contagem() {
        let resultado = conferir_casos(&[
            caso("", "1000.00", "800.00", "1000.00", "800.00"),
            caso("", "1000.00", "800.00", "1000.00", "800.00"),
        ])
        .unwrap();
        assert_eq!(resultado.casos_distintos, 1);
        assert!(!resultado.equivalencia_declarada);
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
