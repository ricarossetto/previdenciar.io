//! Regressões de duas portas que aceitavam entrada incoerente em silêncio.
//!
//! R1 — `conferir_casos` declarava equivalência com quatro cópias dos mesmos
//! valores desde que os rótulos fossem diferentes. A prova de diversidade agora
//! conta tuplas de valores normalizadas, não entradas na lista.
//!
//! R2 — `rgps_core::calcular` devolvia `Ok` com `perfil_correcao` preenchido e
//! `corrigir_salarios=false`, ignorando o perfil. A guarda passou para o núcleo
//! (`validar`), não só para o adaptador.
//!
//! Nenhum teste aqui confirma números de benefício: só rejeição e contagem.
//! Execução pendente (cache de integridade ativo nesta frente).

use rgps_core::calendar::Competencia;
use rgps_core::conferencia::{conferir_casos, ConferenciaCaso, MINIMO_CASOS_PARIDADE};
use rgps_core::engine::{calcular, validar, RgpsEntrada, RgpsOpcoes};
use rgps_core::money::parse_money;

fn caso(rotulo: &str, sb: &str, rmi: &str) -> ConferenciaCaso {
    ConferenciaCaso {
        rotulo: rotulo.into(),
        sb_local: sb.into(),
        rmi_local: rmi.into(),
        sb_oficial: sb.into(),
        rmi_oficial: rmi.into(),
    }
}

fn entrada_300() -> RgpsEntrada {
    RgpsEntrada {
        nome: None,
        especie: None,
        sexo: "feminino".into(),
        data_nascimento: None,
        data_referencia: None,
        vinculos: vec![],
        competencias: (0..300)
            .map(|i| {
                let total = 1994 * 12 + 6 + i as u16;
                (
                    Competencia {
                        ano: total / 12,
                        mes: (total % 12 + 1) as u8,
                    },
                    parse_money("2000.00", "s").unwrap(),
                )
            })
            .collect(),
        opcoes: RgpsOpcoes::default(),
    }
}

// ---------------------------------------------------------------- R1

#[test]
fn quatro_copias_dos_mesmos_valores_nao_declaram_equivalencia() {
    let casos: Vec<_> = (0..MINIMO_CASOS_PARIDADE)
        .map(|i| caso(&format!("sintetico-{i}"), "5083.89", "4067.11"))
        .collect();
    let resultado = conferir_casos(&casos).unwrap();
    assert_eq!(resultado.casos_informados, 4);
    assert_eq!(resultado.casos_distintos, 1);
    assert!(resultado.sb_confere_todos && resultado.rmi_confere_todos);
    assert!(
        !resultado.equivalencia_declarada,
        "quatro cópias com rótulos distintos não são quatro casos: {}",
        resultado.fundamento
    );
}

#[test]
fn quatro_resultados_distintos_conferindo_declaram_equivalencia_do_conjunto() {
    let valores = [
        ("5083.89", "4067.11"),
        ("2000.00", "1600.00"),
        ("1518.00", "1214.40"),
        ("925.93", "555.56"),
    ];
    let casos: Vec<_> = valores
        .iter()
        .enumerate()
        .map(|(i, (sb, rmi))| caso(&format!("c{i}"), sb, rmi))
        .collect();
    let resultado = conferir_casos(&casos).unwrap();
    assert_eq!(resultado.casos_distintos, 4);
    assert!(resultado.equivalencia_declarada, "{}", resultado.fundamento);
    assert!(resultado.fundamento.contains("somente para este conjunto"));
    assert!(
        resultado.fundamento.contains("NÃO comprova cobertura"),
        "a alegação fica limitada ao conjunto: {}",
        resultado.fundamento
    );
}

#[test]
fn variar_so_o_rotulo_nunca_acrescenta_caso() {
    let mut casos = vec![
        caso("a", "1000.00", "800.00"),
        caso("b", "1200.00", "960.00"),
        caso("c", "1400.00", "1120.00"),
    ];
    for i in 0..5 {
        casos.push(caso(&format!("alias-{i}"), "1000.00", "800.00"));
    }
    let resultado = conferir_casos(&casos).unwrap();
    assert_eq!(resultado.casos_informados, 8);
    assert_eq!(resultado.casos_distintos, 3, "aliases não criam tuplas");
    assert!(!resultado.equivalencia_declarada);
}

// ---------------------------------------------------------------- R2

#[test]
fn perfil_com_correcao_desligada_falha_no_nucleo() {
    let mut entrada = entrada_300();
    entrada.opcoes.perfil_correcao = Some("trf3-v1".into());
    entrada.opcoes.corrigir_salarios = false;

    let erro = calcular(&entrada).expect_err("calcular deve bloquear, não ignorar o perfil");
    assert!(erro.0.contains("trf3-v1"), "{erro}");
    assert!(erro.0.contains("corrigir_salarios"), "{erro}");
    // A porta pública de validação concorda com calcular.
    let erro_validar = validar(&entrada).expect_err("validar deve bloquear");
    assert!(erro_validar.0.contains("corrigir_salarios=false"), "{erro_validar}");
}

#[test]
fn perfil_desconhecido_falha_antes_de_calcular() {
    let mut entrada = entrada_300();
    entrada.opcoes.perfil_correcao = Some("perfil-que-nao-existe".into());
    entrada.opcoes.corrigir_salarios = true;
    let erro = validar(&entrada).expect_err("perfil desconhecido deve falhar na validação");
    assert!(erro.0.contains("trf3-v1"), "lista os perfis disponíveis: {erro}");
    assert!(calcular(&entrada).is_err());
}

#[test]
fn fallback_documentado_sem_perfil_falha_na_validacao() {
    let mut entrada = entrada_300();
    entrada.opcoes.fallback_inpc_documentado = true;
    let erro = validar(&entrada).expect_err("fallback sem perfil deve falhar");
    assert!(erro.0.contains("exige perfil de correção"), "{erro}");
}

#[test]
fn perfil_em_branco_e_ausencia_e_nao_bloqueia() {
    let mut entrada = entrada_300();
    entrada.opcoes.perfil_correcao = Some("  ".into());
    entrada.opcoes.corrigir_salarios = false;
    assert!(validar(&entrada).is_ok(), "espaços não formam um perfil");
    let resultado = calcular(&entrada).unwrap();
    assert_eq!(resultado.resumo.rmi, "1600.00", "caminho nominal segue intacto");
}
