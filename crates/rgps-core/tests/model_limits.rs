//! These tests document observed copied-code limitations, NOT correct statutory rules.
//! They do not change/weaken any original anchor assertion.
use rgps_core::{calcular,RgpsEntrada,RgpsOpcoes};
use rgps_core::calendar::{Competencia,IsoDate};
use rgps_core::tempo::{calcular_tempo,VinculoEntrada};
use rust_decimal::Decimal;
fn date(s:&str)->IsoDate{IsoDate::parse(s,"fixture").unwrap()}
#[test] fn fator_previdenciario_reduces_when_expectativa_is_long(){
    let f=rgps_core::transicoes::fator_previdenciario("masculino",120,600,Decimal::from(40)).unwrap();
    assert!(f<Decimal::ONE,"Lei 9.876/1999 aplica o fator ainda que redutor: {f}");
    assert!(f>Decimal::ZERO);
}
#[test] fn transition_time_uses_confirmed_periods_including_pre_1994(){
    let e=RgpsEntrada{nome:None,especie:None,sexo:"feminino".into(),data_nascimento:Some(date("1960-01-01")),data_referencia:Some(date("2025-01-31")),vinculos:vec![VinculoEntrada{inicio:date("1980-01-01"),fim:Some(date("2024-12-31")),tipo:"empregado".into(),empresa:None,fator:1.0}],competencias:vec![(Competencia::parse("2024-12","fixture").unwrap(),Decimal::from(2000))],opcoes:RgpsOpcoes::default()};
    let r=calcular(&e).unwrap();
    let transicoes=r.transicoes.unwrap();
    // O tempo apurado cobre 1980..2024, mesmo com uma única competência no PBC.
    assert!(transicoes["tempo_total_meses"].as_i64().unwrap()>400, "{transicoes}");
    assert!(r.tempo.unwrap()["total_dias_efetivos"].as_i64().unwrap()>10000);
}
#[test] fn overlap_is_deduplicated_in_total_and_reform_markers(){
    let l=VinculoEntrada{inicio:date("2019-01-01"),fim:Some(date("2019-10-31")),tipo:"empregado".into(),empresa:None,fator:1.0};
    let r=calcular_tempo(&[l.clone(),l],Some(&date("2025-01-31"))).unwrap();
    let v=serde_json::to_value(r).unwrap();
    let total=v["total_dias_efetivos"].as_i64().unwrap();
    assert!(total>0);
    assert_eq!(v["marcos"]["ate_ec103"].as_i64().unwrap(),total,"marcos usam os segmentos efetivos, sem dupla contagem");
}
