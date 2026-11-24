use rgps_core::calendar::{Competencia, IsoDate};
use rgps_core::engine::{calcular, RgpsEntrada, RgpsOpcoes};
use rgps_core::money::parse_money;
use rgps_core::tempo::VinculoEntrada;
use rust_decimal::Decimal;

fn comp(v: &str) -> Competencia {
    Competencia::parse(v, "t").unwrap()
}

fn date(v: &str) -> IsoDate {
    IsoDate::parse(v, "t").unwrap()
}

fn sal(v: &str) -> Decimal {
    parse_money(v, "s").unwrap()
}

#[test]
fn descarte_otimo_maximiza_rmi_sem_quebrar_carencia_e_art17() {
    // Homem nascido 1966-01-01, DER 2026-06-01 (idade 60a5m), 432 contribuições
    // sendo 24 salários baixos pós-reforma (R$ 1500) e 408 salários de R$ 5000.
    // Ate 13/11/2019 = 353 meses -> entra na regra do pedágio 100% (60 anos + 35a + pedagio)
    // ou pontos. O descarte ótimo deve descartar os salários de 1500 até manter viabilidade.
    let mut competencias = Vec::new();
    let mut c = comp("1994-07");
    for i in 0..380 {
        let valor = if (320..340).contains(&i) { sal("1500.00") } else { sal("5000.00") };
        competencias.push((c, valor));
        c = c.next();
    }
    let sem_descarte = calcular(&RgpsEntrada {
        nome: None,
        especie: Some("42".into()),
        sexo: "masculino".into(),
        data_nascimento: Some(date("1960-01-01")),
        data_referencia: Some(date("2026-06-01")),
        vinculos: vec![VinculoEntrada {
            inicio: date("1990-01-01"),
            fim: Some(date("2026-05-31")),
            tipo: "empregado".into(),
            empresa: None,
            fator: 1.0,
        }],
        competencias: competencias.clone(),
        opcoes: RgpsOpcoes {
            corrigir_salarios: true,
            referencia_correcao: Some(comp("2026-06")),
            descarte_otimo: false,
            ..RgpsOpcoes::default()
        },
    })
    .unwrap();

    let otimo = calcular(&RgpsEntrada {
        nome: None,
        especie: Some("42".into()),
        sexo: "masculino".into(),
        data_nascimento: Some(date("1960-01-01")),
        data_referencia: Some(date("2026-06-01")),
        vinculos: vec![VinculoEntrada {
            inicio: date("1990-01-01"),
            fim: Some(date("2026-05-31")),
            tipo: "empregado".into(),
            empresa: None,
            fator: 1.0,
        }],
        competencias,
        opcoes: RgpsOpcoes {
            corrigir_salarios: true,
            referencia_correcao: Some(comp("2026-06")),
            descarte_otimo: true,
            ..RgpsOpcoes::default()
        },
    })
    .unwrap();

    let rmi_sem = sal(&sem_descarte.resumo.rmi);
    let rmi_otimo = sal(&otimo.resumo.rmi);
    assert!(
        rmi_otimo > rmi_sem,
        "descarte ótimo ({rmi_otimo}) deveria superar sem descarte ({rmi_sem})"
    );
    assert!(!otimo.simulacao_descartes.is_empty());
}

#[test]
fn direito_adquirido_calcula_e_reajusta_ate_dib() {
    // Homem nascido 1954-01-01, 35 anos ate 13/11/2019 (1984-01 a 2019-11), DER 2023-06-30.
    let mut competencias = Vec::new();
    let mut c = comp("1994-07");
    while c <= comp("2023-05") {
        competencias.push((c, sal("4000.00")));
        c = c.next();
    }
    let resultado = calcular(&RgpsEntrada {
        nome: None,
        especie: Some("42".into()),
        sexo: "masculino".into(),
        data_nascimento: Some(date("1954-01-01")),
        data_referencia: Some(date("2023-06-30")),
        vinculos: vec![VinculoEntrada {
            inicio: date("1984-01-01"),
            fim: Some(date("2023-05-31")),
            tipo: "empregado".into(),
            empresa: None,
            fator: 1.0,
        }],
        competencias,
        opcoes: RgpsOpcoes {
            corrigir_salarios: true,
            referencia_correcao: Some(comp("2023-06")),
            aplicar_teto_vigente_der: true,
            aplicar_piso_automatico: true,
            ..RgpsOpcoes::default()
        },
    })
    .unwrap();

    let transicoes = resultado.transicoes.unwrap();
    let da = transicoes["regras"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "direito-adquirido")
        .unwrap();
    assert_eq!(da["elegivel"], true);
    assert_eq!(da["criterio_rmi"], "direito-adquirido-reajustado");
    assert!(da["rmi"].as_str().is_some());
    assert!(da["rmi_final"].as_str().is_some());
}

#[test]
fn referencia_2026_11_funciona_com_ultimo_indice_publicado_e_piso_automatico() {
    let mut competencias = Vec::new();
    let mut c = comp("2015-01");
    for _ in 0..24 {
        competencias.push((c, sal("900.00")));
        c = c.next();
    }
    let resultado = calcular(&RgpsEntrada {
        nome: None,
        especie: Some("41".into()),
        sexo: "masculino".into(),
        data_nascimento: Some(date("1955-01-01")),
        data_referencia: Some(date("2026-11-17")),
        vinculos: vec![VinculoEntrada {
            inicio: date("2005-01-01"),
            fim: Some(date("2026-10-31")),
            tipo: "empregado".into(),
            empresa: None,
            fator: 1.0,
        }],
        competencias,
        opcoes: RgpsOpcoes {
            corrigir_salarios: true,
            referencia_correcao: Some(comp("2026-11")),
            aplicar_piso_automatico: true,
            aplicar_teto_vigente_der: true,
            divisor_minimo: true,
            ..RgpsOpcoes::default()
        },
    })
    .unwrap();

    // Deve corrigir até o último INPC publicado sem bloquear e aplicar o piso vigente (> 1400).
    let correcao = resultado.correcao.unwrap();
    assert!(correcao["referencia_efetiva"].is_string());
    assert!(correcao["divisor_usado"].as_u64() == Some(108));
    assert_eq!(resultado.resumo.limite_aplicado, Some("piso"));
    assert!(sal(&resultado.resumo.rmi) >= sal("1500.00"));
}
