//! Casos-âncora do motor RGPS — portados de tests/calculation_previdenciario.mjs (ATRIUM)
//! e do spec specs/calculations-previdenciario.md.

use rgps_core::calendar::{Competencia, IsoDate};
use rgps_core::engine::{calcular, RgpsEntrada, RgpsOpcoes};
use rgps_core::money::parse_money;
use rgps_core::projecao::{projetar, ProjecaoRequisicao};
use rgps_core::tempo::{calcular_tempo, VinculoEntrada};

fn comp(v: &str) -> Competencia {
    Competencia::parse(v, "competência").unwrap()
}

fn sequencia(inicio: &str, quantidade: u32, salario: &str) -> Vec<(Competencia, rust_decimal::Decimal)> {
    let valor = parse_money(salario, "salário").unwrap();
    let mut lista = Vec::new();
    let mut cursor = comp(inicio);
    for _ in 0..quantidade {
        lista.push((cursor, valor));
        cursor = cursor.next();
    }
    lista
}

fn entrada_base(sexo: &str, quantidade: u32, salario: &str) -> RgpsEntrada {
    RgpsEntrada {
        nome: Some("Cliente Teste".into()),
        sexo: sexo.into(),
        data_nascimento: None,
        data_referencia: None,
        vinculos: vec![],
        competencias: sequencia("1994-07", quantidade, salario),
        opcoes: RgpsOpcoes::default(),
    }
}

#[test]
fn mulher_300_competencias_de_2000_rmi_1600() {
    let resultado = calcular(&entrada_base("feminino", 300, "2000.00")).unwrap();
    // 25 anos → 60% + 2%×10 = 80%; SB 2000 → RMI 1600.
    assert_eq!(resultado.resumo.salario_beneficio, "2000.00");
    assert_eq!(resultado.resumo.coeficiente, "80");
    assert_eq!(resultado.resumo.rmi, "1600.00");
    assert_eq!(resultado.resumo.carencia_meses, 300);
}

#[test]
fn homem_240_competencias_de_1500_rmi_900() {
    let resultado = calcular(&entrada_base("masculino", 240, "1500.00")).unwrap();
    // 20 anos → coeficiente 60%; 1500 × 0,6 = 900.
    assert_eq!(resultado.resumo.coeficiente, "60");
    assert_eq!(resultado.resumo.rmi, "900.00");
}

#[test]
fn quatorze_anos_mantem_coeficiente_60_com_aviso() {
    let mut entrada = entrada_base("feminino", 168, "2000.00");
    entrada.opcoes.corrigir_salarios = false;
    let resultado = calcular(&entrada).unwrap();
    assert_eq!(resultado.resumo.coeficiente, "60");
    assert!(resultado
        .avisos
        .iter()
        .any(|a| a.contains("abaixo do mínimo")));
}

#[test]
fn teto_aplicado_com_memoria_limite() {
    let mut entrada = entrada_base("feminino", 300, "2000.00");
    entrada.opcoes.teto = Some(parse_money("1500.00", "teto").unwrap());
    let resultado = calcular(&entrada).unwrap();
    assert_eq!(resultado.resumo.limite_aplicado, Some("teto"));
    assert_eq!(resultado.resumo.rmi, "1500.00");
    assert!(resultado.memoria.iter().any(|m| m.tipo == "limite"));
}

#[test]
fn piso_aplicado_sobre_rmi_baixa() {
    let mut entrada = entrada_base("feminino", 12, "1000.00");
    entrada.opcoes.piso = Some(parse_money("1518.00", "piso").unwrap());
    let resultado = calcular(&entrada).unwrap();
    assert_eq!(resultado.resumo.limite_aplicado, Some("piso"));
    assert_eq!(resultado.resumo.rmi, "1518.00");
}

#[test]
fn divisor_minimo_108_com_50_competencias() {
    let mut entrada = entrada_base("masculino", 50, "1000.00");
    entrada.opcoes.divisor_minimo = true;
    let resultado = calcular(&entrada).unwrap();
    // 50.000 / 108 = 462,96 (half-up).
    assert_eq!(resultado.resumo.salario_beneficio, "462.96");
    assert!(resultado
        .avisos
        .iter()
        .any(|a| a.contains("Divisor mínimo aplicado")));
}

#[test]
fn correcao_ativa_com_teto_da_epoca_e_descarte() {
    // 10 competências em 2004, salários que estouram o teto 2.400 de 2004-01.
    let competencias: Vec<_> = (1..=10)
        .map(|m| {
            let mes = if m < 10 { format!("2004-0{m}") } else { "2004-10".into() };
            (comp(&mes), parse_money("5000.00", "s").unwrap())
        })
        .collect();
    let entrada = RgpsEntrada {
        nome: None,
        sexo: "feminino".into(),
        data_nascimento: None,
        data_referencia: None,
        vinculos: vec![],
        competencias,
        opcoes: RgpsOpcoes {
            corrigir_salarios: true,
            descarte: true,
            referencia_correcao: Some(comp("2025-02")),
            ..RgpsOpcoes::default()
        },
    };
    let resultado = calcular(&entrada).unwrap();
    let correcao = resultado.correcao.as_ref().unwrap();
    // O teto da época é aplicado antes da correção em todas as parcelas.
    for parcela in correcao.get("parcelas").unwrap().as_array().unwrap() {
        let teto = parcela.get("teto").unwrap().as_str().unwrap();
        assert_eq!(parcela.get("considerado").unwrap().as_str().unwrap(), teto);
    }
    assert_eq!(
        correcao.get("parcelas").unwrap().as_array().unwrap()[0]
            .get("teto")
            .unwrap(),
        &serde_json::json!("2400.00")
    );
    // Descarte 20% de 10 = 2 descartadas, 8 consideradas.
    assert_eq!(correcao.get("consideradas").unwrap(), &serde_json::json!(8));
    assert_eq!(correcao.get("descartadas").unwrap(), &serde_json::json!(2));
}

#[test]
fn guardas_de_entrada() {
    // Anterior a 1994-07.
    let mut entrada = entrada_base("feminino", 1, "1000.00");
    entrada.competencias = vec![(comp("1994-06"), parse_money("100.00", "s").unwrap())];
    assert!(calcular(&entrada).is_err());

    // Duplicada.
    let mut entrada = entrada_base("feminino", 2, "1000.00");
    entrada.competencias[1].0 = entrada.competencias[0].0;
    assert!(calcular(&entrada).is_err());

    // Salário zero.
    let mut entrada = entrada_base("feminino", 1, "0.00");
    assert!(calcular(&entrada).is_err());

    // Piso maior que teto.
    let mut entrada = entrada_base("feminino", 12, "1000.00");
    entrada.opcoes.teto = Some(parse_money("100.00", "teto").unwrap());
    entrada.opcoes.piso = Some(parse_money("200.00", "piso").unwrap());
    assert!(calcular(&entrada).is_err());
}

#[test]
fn tempo_com_vinculos_e_marcos() {
    let vinculos = vec![
        VinculoEntrada {
            inicio: IsoDate::parse("2015-01-01", "t").unwrap(),
            fim: Some(IsoDate::parse("2019-11-13", "t").unwrap()),
            tipo: "empregado".into(),
            empresa: Some("Empresa Teste".into()),
            fator: 1.0,
        },
        VinculoEntrada {
            inicio: IsoDate::parse("2019-11-14", "t").unwrap(),
            fim: Some(IsoDate::parse("2026-10-09", "t").unwrap()),
            tipo: "contribuinte_individual".into(),
            empresa: None,
            fator: 1.0,
        },
    ];
    let tempo = calcular_tempo(&vinculos, None).unwrap();
    assert!(tempo.total_dias > 0);
    assert!(tempo.marcos.ate_ec103 > 0);
    assert!(tempo.marcos.apos_ec103 > 0);
}

#[test]
fn transicoes_ec103_com_nascimento() {
    let mut entrada = entrada_base("masculino", 420, "2000.00");
    // Nascido 1958-03-10: idade 68a6m + 35a de tempo = 103 pontos em 2026 (o exigido).
    entrada.data_nascimento = Some(IsoDate::parse("1958-03-10", "t").unwrap());
    entrada.data_referencia = Some(IsoDate::parse("2026-10-09", "t").unwrap());
    let resultado = calcular(&entrada).unwrap();
    let transicoes = resultado.transicoes.as_ref().unwrap();
    let regras = transicoes.get("regras").unwrap().as_array().unwrap();
    assert_eq!(regras.len(), 6);
    let pontos = regras
        .iter()
        .find(|r| r.get("id").unwrap() == "pontos")
        .unwrap();
    assert_eq!(pontos.get("elegivel").unwrap(), &serde_json::json!(true));
    // RMI da regra de pontos: SB 2000 × coeficiente (35 anos → 90%) = 1800.
    assert_eq!(pontos.get("rmi").unwrap(), &serde_json::json!("1800.00"));
    let pedagio50 = regras
        .iter()
        .find(|r| r.get("id").unwrap() == "pedagio-50")
        .unwrap();
    assert_eq!(
        pedagio50.get("faltas").unwrap().get("aplicavel"),
        Some(&serde_json::json!(false))
    );
    // Vencedora entre elegíveis calculáveis.
    assert!(transicoes.get("regra_escolhida").unwrap().is_string());
}

#[test]
fn projecao_homem_projeta_der() {
    let req = ProjecaoRequisicao {
        sexo: "masculino".into(),
        data_nascimento: IsoDate::parse("1966-05-10", "t").unwrap(),
        data_referencia: IsoDate::parse("2026-10-09", "t").unwrap(),
        competencias: sequencia("1996-06", 360, "3000.00"),
        corrigir_salarios: false,
        descarte: false,
        salario_simulacao: None,
        teto: None,
        piso: None,
        aplicar_teto_vigente_der: false,
        expectativa_sobrevida: None,
        regra_alvo: None,
    };
    let resultado = projetar(&req).unwrap();
    assert!(resultado.der.is_some(), "deve projetar DER: {resultado:?}");
    assert!(resultado.rmi.is_some());
    assert!(resultado.idade_der_meses.unwrap() >= 60 * 12);
}
