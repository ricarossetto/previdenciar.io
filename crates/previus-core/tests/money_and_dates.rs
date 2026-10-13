use previus_core::{dates::{count_inclusive_calendar_days, InclusiveIntervalInput}, CivilDate, DomainError, Money, RoundingMode, YearMonth};
use std::str::FromStr;

#[test] fn exact_br_formats_round_trip() {
    for (input, expected) in [("R$ 1.234,56",123456), ("1234,56",123456), ("-0,01",-1),
        ("0",0), ("1,2",120), ("1.234",123400), (" R$ -12,34 ",-1234)] {
        let value = Money::parse_br(input).unwrap();
        assert_eq!(value.cents(),expected);
        assert_eq!(Money::parse_br(&value.to_br()).unwrap(),value);
    }
}
#[test] fn locale_rejects_ambiguity_and_bad_grouping() {
    for input in ["1,234.56","12.34,56","1.23.456,78",".123,45","1.234,","1,000", "NaN", "1e3", "+1,00", "1 234,56", "１２,３４", "", "-", "1,2,3"] {
        assert!(Money::parse_br(input).is_err(), "accepted invalid input: {input}");
    }
}
#[test] fn decimal_wire_rejects_floats_scientific_and_precision_loss() {
    for input in ["1,23","1.234","1e2",".5","1."," 1.00 ","+1.00","NaN","∞"] {
        assert!(Money::parse_decimal(input).is_err(),"accepted {input}");
    }
    assert_eq!(Money::parse_decimal("-0.00").unwrap().to_decimal(),"0.00");
    assert_eq!(Money::parse_decimal("0001.2").unwrap().to_decimal(),"1.20");
}
#[test] fn min_max_money_round_trip_and_overflow() {
    for cents in [i64::MIN, i64::MAX, 0, -1, 1, 1000000000001] {
        let value = Money::from_cents(cents);
        assert_eq!(Money::parse_decimal(&value.to_decimal()).unwrap(),value);
        assert_eq!(Money::parse_br(&value.to_br()).unwrap(),value);
    }
    assert_eq!(Money::parse_decimal("92233720368547758.08"),Err(DomainError::Overflow));
    assert_eq!(Money::parse_decimal("-92233720368547758.09"),Err(DomainError::Overflow));
    assert_eq!(Money::from_cents(i64::MAX).checked_add(Money::from_cents(1)),Err(DomainError::Overflow));
    assert_eq!(Money::from_cents(i64::MIN).checked_sub(Money::from_cents(1)),Err(DomainError::Overflow));
    assert_eq!(Money::from_cents(i64::MAX).apply_percent(255,RoundingMode::HalfAwayFromZero),Err(DomainError::Overflow));
}
#[test] fn rounding_is_explicit_and_symmetric() {
    assert_eq!(Money::from_cents(5).apply_percent(10,RoundingMode::HalfAwayFromZero).unwrap().cents(),1);
    assert_eq!(Money::from_cents(-5).apply_percent(10,RoundingMode::HalfAwayFromZero).unwrap().cents(),-1);
    assert_eq!(Money::from_cents(5).apply_percent(10,RoundingMode::TowardZero).unwrap().cents(),0);
    assert_eq!(Money::from_cents(-5).apply_percent(10,RoundingMode::TowardZero).unwrap().cents(),0);
}
#[test] fn json_money_is_exact_string_brl() {
    let money = Money::from_cents(i64::MAX);
    let json = serde_json::to_value(money).unwrap();
    assert_eq!(json["currency"],"BRL");
    assert!(json["amount"].is_string());
    assert_eq!(serde_json::from_value::<Money>(json).unwrap(),money);
    for bad in [r#"{"currency":"BRL","amount":1.23}"#, r#"{"currency":"USD","amount":"1.23"}"#,
        r#"{"currency":"BRL","amount":"1.234"}"#,r#"{"currency":"BRL","amount":"1.23","cents":123}"#] {
        assert!(serde_json::from_str::<Money>(bad).is_err());
    }
}
#[test] fn many_cent_values_round_trip_without_float() {
    for cents in -10000..=10000 {
        let value = Money::from_cents(cents);
        assert_eq!(Money::parse_br(&value.to_br()).unwrap(), value);
        assert_eq!(Money::parse_decimal(&value.to_decimal()).unwrap(), value);
    }
}
#[test] fn gregorian_leap_date_rules() {
    assert!(CivilDate::new(2024,2,29).is_ok());
    assert!(CivilDate::new(2000,2,29).is_ok());
    for year in [2023,1900,2100] { assert_eq!(CivilDate::new(year,2,29),Err(DomainError::InvalidDate)); }
    assert_eq!(CivilDate::new(2024,4,31),Err(DomainError::InvalidDate));
}
#[test] fn invalid_civil_inputs_cannot_cross_json_boundary() {
    for s in ["2024-02-30","2024-00-01","2024-13-01","0000-01-01","10000-01-01","2024-2-09",
        "2024-01-00","2024-01-01T00:00:00Z","２０２４-01-01","2024-0é-01"] {
        assert!(CivilDate::from_str(s).is_err());
        assert!(serde_json::from_value::<CivilDate>(serde_json::json!(s)).is_err());
    }
    assert!(CivilDate::parse_br("31/02/2024").is_err());
    assert!(CivilDate::parse_br("1/02/2024").is_err());
    assert!(CivilDate::parse_br("2024-02-29").is_err());
}
#[test] fn civil_date_and_competence_string_round_trip() {
    let d = CivilDate::parse_br("29/02/2024").unwrap();
    assert_eq!(d.to_string(),"2024-02-29");
    assert_eq!(d.to_br(),"29/02/2024");
    assert_eq!(serde_json::from_str::<CivilDate>(&serde_json::to_string(&d).unwrap()).unwrap(),d);
    let ym = YearMonth::new(2026,10).unwrap();
    assert_eq!(serde_json::to_string(&ym).unwrap(),r#""2026-10""#);
    assert_eq!(ym.checked_add_months(6).unwrap().to_string(),"2027-04");
    assert_eq!(YearMonth::new(2024,2).unwrap().checked_add_months(6).unwrap().to_string(),"2024-08");
}
#[test] fn month_and_date_limits_fail_without_panics() {
    assert_eq!(YearMonth::new(9999,12).unwrap().checked_add_months(1),Err(DomainError::Overflow));
    assert_eq!(YearMonth::new(2026,10).unwrap().checked_add_months(u32::MAX),Err(DomainError::Overflow));
    assert_eq!(CivilDate::new(9999,12,31).unwrap().next_day(),Err(DomainError::Overflow));
    for s in ["0000-01","2026-00","2026-13","2026-1","2026-01-01","２０２６-01"] {
        assert!(s.parse::<YearMonth>().is_err());
    }
}
#[test] fn inclusive_is_named_policy_not_general_contribution_rule() {
    let output = count_inclusive_calendar_days(&InclusiveIntervalInput {
        start: "2024-02-28".parse().unwrap(), end:"2024-03-01".parse().unwrap()
    }).unwrap().data.unwrap();
    assert_eq!(output.calendar_days,3);
    assert_eq!(output.counting_policy,"both_endpoints_included");
    assert_eq!(output.duration_text,"3 dias");
}
#[test] fn reversed_intervals_are_rejected() {
    assert_eq!(count_inclusive_calendar_days(&InclusiveIntervalInput {
        start:"2026-10-11".parse().unwrap(),end:"2026-10-10".parse().unwrap()
    }),Err(DomainError::ReversedInterval));
}
