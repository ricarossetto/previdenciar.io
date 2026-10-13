//! These tests document observed copied-code limitations, NOT correct statutory rules.
//! They do not change/weaken any original anchor assertion.
use rgps_core::{calcular,RgpsEntrada,RgpsOpcoes};
use rgps_core::calendar::{Competencia,IsoDate};
use rgps_core::tempo::{calcular_tempo,VinculoEntrada};
use rust_decimal::Decimal;
fn date(s:&str)->IsoDate{IsoDate::parse(s,"fixture").unwrap()}
#[test] fn known_fator_floor_one_is_explicit_observed_legacy_limit(){
    let f=rgps_core::transicoes::fator_previdenciario("masculino",120,600,Decimal::from(40)).unwrap();
    assert_eq!(f,Decimal::ONE,"copied source floors a reducing factor; NOT validated legal behavior");
}
#[test] fn known_transition_time_ignores_confirmed_pre_1994_periods(){
    let e=RgpsEntrada{nome:None,sexo:"feminino".into(),data_nascimento:Some(date("1960-01-01")),data_referencia:Some(date("2025-01-31")),vinculos:vec![VinculoEntrada{inicio:date("1980-01-01"),fim:Some(date("2024-12-31")),tipo:"empregado".into(),empresa:None,fator:1.0}],competencias:vec![(Competencia::parse("2024-12","fixture").unwrap(),Decimal::from(2000))],opcoes:RgpsOpcoes::default()};
    let r=calcular(&e).unwrap();assert_eq!(r.transicoes.unwrap()["tempo_total_meses"],1);
    assert!(r.tempo.unwrap()["total_dias_efetivos"].as_i64().unwrap()>10000);
}
#[test] fn known_overlap_is_deduplicated_total_but_not_reform_markers(){
    let l=VinculoEntrada{inicio:date("2019-01-01"),fim:Some(date("2019-10-31")),tipo:"empregado".into(),empresa:None,fator:1.0};
    let r=calcular_tempo(&[l.clone(),l],Some(&date("2025-01-31"))).unwrap();
    let v=serde_json::to_value(r).unwrap();
    let total=v["total_dias_efetivos"].as_i64().unwrap();
    assert!(total>0);
    assert_eq!(v["marcos"]["ate_ec103"].as_i64().unwrap(),total*2, "original markers sum overlapping periods twice");
}
