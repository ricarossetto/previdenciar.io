//! Pure, fail-closed adapter. No network, database, filesystem writes, UI or inference.
//! Native results are observational research output, NOT verified benefit entitlement.
#![forbid(unsafe_code)]
mod bridge;
pub mod pcd;
pub mod guided;
pub mod contracts;
pub use bridge::calculate_case;
pub use guided::prepare_from_cnis;
use contracts::*;
use rgps_core::{RgpsEntrada, RgpsOpcoes};
use rgps_core::calendar::{Competencia, IsoDate};
use rgps_core::tempo::VinculoEntrada;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub const POLICY_VERSION: &str = "rgps-core-observational-v1";
pub const PUBLIC_ECONOMIC_JSON: &str = include_str!("public-economic.json");
const MAX_LINKS: usize = 2000;
const MAX_WAGES: usize = 2400;

/// Machine-readable UI template. Null review/category/currency/options are
/// intentional blockers until explicit operator action; this is NOT approval.
pub fn required_policy_example() -> Value {
    let mut policy = contracts::CasePolicy::default();
    policy.version = Some(POLICY_VERSION.into());
    serde_json::to_value(policy).expect("case policy template")
}

pub fn capabilities() -> Value {
    let tables: Value = serde_json::from_str(PUBLIC_ECONOMIC_JSON).expect("embedded audited JSON");
    json!({
        "policy_version": POLICY_VERSION,
        "engine": "rgps-core", "engine_version": rgps_core::ENGINE_VERSION,
        "legal_validation_performed": false,
        "implemented_native": ["pbc", "inpc_correction", "ceilings", "contribution_time", "carencia", "rmi_observational", "transitions", "projection_typed_core_api"],
        "adapter_supported": ["calculate_json", "calculate_case_manual_confirmed"],
        "unsupported": ["automatic_cnis_validation", "disability_rules", "rural_special_rules", "automatic_ctc",  "historic_currency_conversion", "old_law_rmi", "statutory_entitlement", "projection_json"],
        "known_model_risks": model_risks(),
        "tables": {
            "inpc": {"first": tables["inpc"]["first"], "last": tables["inpc"]["last"], "count": tables["inpc"]["count"], "source_declaration": tables["inpc"]["source_declaration"]},
            "igpdi": {"first": tables["igpdi"]["first"], "last": tables["igpdi"]["last"], "count": tables["igpdi"]["count"], "source_declaration": tables["igpdi"]["source_declaration"]},
            "ceilings": {"first": tables["ceilings"]["first"], "last_change": tables["ceilings"]["last"], "count": tables["ceilings"]["count"], "source_declaration": tables["ceilings"]["source_declaration"]},
            "official_values_independently_verified": false
        }
    })
}
fn model_risks() -> Vec<&'static str> {
    vec![
        "points_truncate_age_and_time_to_whole_years",
        "grace_and_late_payments_model_not_validated_and_adapter_blocks_them",
        "pre_reform_rmi_not_recalculated",
        "discard_and_divisor_are_operator_options_not_date_selected_law",
        "native_corrected_pbc_does_not_apply_minimum_divisor_option",
        "tables_are_owned_source_snapshot_not_freshly_verified_official_values",
    ]
}
pub(crate) fn blocked(items: Vec<Blocker>, warnings: Vec<String>) -> Value {
    json!({"status":"blocked", "rule_review_status":"unvalidated", "policy_version":POLICY_VERSION,
        "legal_validation_performed":false, "data":null,
        "missing": items.iter().map(|b| b.code.clone()).collect::<Vec<_>>(), "blockers":items,
        "warnings": warnings, "capabilities": capabilities()})
}
pub(crate) fn issue(out: &mut Vec<Blocker>, code: &str, field: &str, message: &str) {
    out.push(Blocker { code:code.into(), field:field.into(), message:message.into() });
}
fn nonempty(v: &Option<String>) -> bool { v.as_ref().is_some_and(|s| !s.trim().is_empty()) }
fn date(v: &Option<String>, field: &str, out: &mut Vec<Blocker>) -> Option<IsoDate> {
    let Some(s) = v.as_ref().filter(|s| s.is_ascii()) else {
        issue(out,"missing_or_invalid_date",field,"Data obrigatória, ISO AAAA-MM-DD; não inferida."); return None;
    };
    match IsoDate::parse(s,field) {
        Ok(d) if (1900..=2100).contains(&d.ano) => Some(d),
        _ => { issue(out,"missing_or_invalid_date",field,"Data inválida ou fora do intervalo operacional 1900 a 2100."); None }
    }
}
fn month(v: &Option<String>, field: &str, out: &mut Vec<Blocker>) -> Option<Competencia> {
    let Some(s) = v.as_ref().filter(|s| s.is_ascii()) else {
        issue(out,"missing_or_invalid_month",field,"Competência obrigatória, AAAA-MM."); return None;
    };
    match Competencia::parse(s,field) {
        Ok(c) if (1900..=2100).contains(&c.ano) => Some(c),
        _ => { issue(out,"missing_or_invalid_month",field,"Competência inválida ou fora do intervalo operacional."); None }
    }
}
fn amount(v: &Option<String>, field: &str, out: &mut Vec<Blocker>, max: Decimal) -> Option<Decimal> {
    let Some(s) = v.as_ref() else { issue(out,"missing_amount",field,"Valor decimal em string obrigatório."); return None; };
    let valid_syntax = !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit() || c == b'.') && s.matches('.').count() <= 1
        && s.split('.').next().is_some_and(|v| !v.is_empty()) && s.split('.').nth(1).is_none_or(|v| !v.is_empty() && v.len() <= 2);
    match Decimal::from_str_exact(s).ok().filter(|d| valid_syntax && *d > Decimal::ZERO && *d <= max) {
        Some(d) => Some(d),
        None => { issue(out,"invalid_amount",field,"Valor positivo em string decimal com ponto e até 2 casas; sem notação científica, float, separador local ou overflow."); None }
    }
}

/// Salário mínimo vigente na competência, pela série oficial embutida.
/// Sem cobertura para a competência, não há exclusão (não se inventa piso).
fn salario_minimo_vigente(competencia: &Competencia) -> Option<Decimal> {
    let chave = competencia.to_string();
    rgps_core::dados::salario_minimo::SALARIO_MINIMO
        .iter()
        .filter(|(periodo, _)| *periodo <= chave.as_str())
        .last()
        .and_then(|(_, valor)| Decimal::from_str_exact(valor).ok())
}

/// Malformed/unknown JSON fields return Err. Missing/unsafe domain data returns a
/// blocked envelope with data:null. Successful output is explicitly experimental.
pub fn calculate_json(input: Value) -> Result<Value, String> {
    if !input.is_object() { return Err("invalid_input_schema: expected JSON object".into()); }
    let request: CalculationInput = serde_json::from_value(input).map_err(|e| format!("invalid_input_schema: {e}"))?;
    calculate_input(request)
}

fn calculate_input(r: CalculationInput) -> Result<Value, String> {
    let mut b = Vec::new();
    let mut warnings = vec!["Resultado experimental do código próprio copiado; sem validação normativa ou equivalência comprovada com o Previus.".into()];
    if !r.revisao.confirmada || !nonempty(&r.revisao.evidencia) {
        issue(&mut b,"review_not_confirmed","revisao","Confirmar revisão documental e informar referência de evidência; não usar identidade pessoal como referência.");
    }
    if r.revisao.politica.as_deref() != Some(POLICY_VERSION) {
        issue(&mut b,"policy_not_selected","revisao.politica","Política observacional explícita obrigatória.");
    }
    let sex = r.segurado.sexo.as_deref().unwrap_or("").trim().to_lowercase();
    if sex != "feminino" && sex != "masculino" {
        issue(&mut b,"sex_unconfirmed","segurado.sexo","Sexo cadastral obrigatório; não inferir pelo nome.");
    }
    let disability = r.segurado.deficiencia.as_deref().unwrap_or("").trim().to_lowercase();
    if !["nao", "não", "none", "sem_deficiencia", "não possui deficiência", "nao possui deficiencia"].contains(&disability.as_str()) {
        issue(&mut b, if disability.is_empty() {"disability_unconfirmed"} else {"disability_rules_unsupported"},"segurado.deficiencia","Ausência de deficiência deve ser explicitamente confirmada; regras PcD não implementadas neste adaptador.");
    }
    let birth = date(&r.segurado.data_nascimento,"segurado.dataNascimento",&mut b);
    let dib = date(&r.data_referencia,"dataReferencia",&mut b);
    if let (Some(birth),Some(dib)) = (birth,dib) {
        if birth >= dib { issue(&mut b,"birth_after_dib","segurado.dataNascimento","Nascimento deve anteceder a DIB."); }
        if dib < (IsoDate {ano:2019,mes:11,dia:13}) {
            issue(&mut b,"historical_policy_unsupported","dataReferencia","Adaptador restrito a cenários pós-reforma; cálculo de RMI de legislação anterior não validado.");
        }
    }
    if r.vinculos.is_empty() { issue(&mut b,"links_missing","vinculos","Períodos de contribuição com datas finais confirmadas são obrigatórios."); }
    if r.vinculos.len() > MAX_LINKS || r.competencias.len() > MAX_WAGES {
        issue(&mut b,"input_limit","vinculos/competencias","Limite de registros excedido.");
        return Ok(blocked(b,warnings));
    }
    let mut links = Vec::new();
    for (i,l) in r.vinculos.iter().enumerate() {
        let f = format!("vinculos[{i}]");
        let start = date(&l.inicio,&format!("{f}.inicio"),&mut b);
        let end = date(&l.fim,&format!("{f}.fim"),&mut b);
        if !l.confirmado || !nonempty(&l.evidencia) { issue(&mut b,"link_unconfirmed",&f,"Período exige confirmação manual e referência documental explícitas."); }
        if !l.indicadores.is_empty() { issue(&mut b,"link_indicators_unresolved",&f,"Indicadores pendentes não podem ser descartados pelo adaptador."); }
        let regime = l.regime.as_deref().unwrap_or("").trim().to_uppercase();
        let ctc = l.ctc_confirmada && nonempty(&l.ctc_evidencia) && l.origem.as_deref() == Some("manual_documentado") && l.confirmado;
        if regime != "RGPS" && !(["RPPS","PRPPS"].contains(&regime.as_str()) && ctc) {
            issue(&mut b,"regime_unvalidated",&f,"Regime deve ser RGPS confirmado ou período manual com CTC documental explicitamente confirmada.");
        }
        if (l.prpps || ["RPPS","PRPPS"].contains(&regime.as_str())) && !ctc {
            issue(&mut b,"prpps_unvalidated",&f,"PRPPS/RPPS não é contado automaticamente: exigir período manual documentado e CTC confirmada.");
        }
        if ctc { warnings.push("Período CTC manual declarado pelo operador; enquadramento e ausência de dupla contagem não validados juridicamente.".into()); }
        let kind = l.tipo.as_deref().unwrap_or("");
        if !["empregado","empregado_domestico","contribuinte_individual","avulso","facultativo"].contains(&kind) {
            issue(&mut b,"link_kind_unsupported",&f,"Categoria deve ser explícita; tempo especial, rural, servidor sem CTC e PcD não são convertidos automaticamente.");
        }
        if l.fator.is_none_or(|n| n != 1.0 || !n.is_finite()) {
            issue(&mut b,"conversion_factor_unsupported",&f,"Fator 1.0 explícito obrigatório; conversões não validadas não são aplicadas.");
        }
        if let (Some(start),Some(end)) = (start,end) {
            if end < start { issue(&mut b,"link_dates_reversed",&f,"Período invertido."); }
            if dib.is_some_and(|d| end > d) { issue(&mut b,"source_end_after_dib",&f,"Data final posterior à DIB; não truncar automaticamente."); }
            if birth.is_some_and(|d| start < d) { issue(&mut b,"source_start_before_birth",&f,"Período anterior ao nascimento."); }
            links.push(VinculoEntrada {inicio:start,fim:Some(end),tipo:kind.into(),empresa:None,fator:1.0});
        }
    }
    let mut wages = Vec::new();
    let mut seen = BTreeSet::new();
    let mut pre94 = 0;
    let mut abaixo_do_minimo: Vec<String> = Vec::new();
    for (i,w) in r.competencias.iter().enumerate() {
        let f = format!("competencias[{i}]");
        let Some(c) = month(&w.competencia,&f,&mut b) else { continue; };
        if c < (Competencia {ano:1994,mes:7}) { pre94+=1; continue; }
        if w.moeda.as_deref() != Some("BRL") { issue(&mut b,"currency_unverified",&f,"PBC desde 07/1994 exige remuneração nominal BRL explicitamente conferida; não converter moeda histórica."); }
        if !w.confirmada { issue(&mut b,"wage_unconfirmed",&f,"Salário nominal exige confirmação documental."); }
        if !w.indicadores.is_empty() { issue(&mut b,"wage_indicators_unresolved",&f,"Indicadores de remuneração exigem resolução documentada."); }
        if !seen.insert(c) { issue(&mut b,"duplicate_month",&f,"Concomitância: competência duplicada; não somar automaticamente."); }
        if dib.is_some_and(|d| c > (Competencia {ano:d.ano as u16,mes:d.mes})) { issue(&mut b,"wage_after_dib",&f,"Remuneração posterior à DIB."); }
        let covered = links.iter().any(|l| c >= Competencia {ano:l.inicio.ano as u16,mes:l.inicio.mes} && l.fim.is_some_and(|e| c <= Competencia {ano:e.ano as u16,mes:e.mes}));
        if !covered { issue(&mut b,"wage_outside_confirmed_links",&f,"Competência sem período confirmado correspondente."); }
        if let Some(a)=amount(&w.salario,&f,&mut b,Decimal::from(1_000_000_000u64)) {
            // Regra do CNIS: a competência só é reconhecida como tempo quando o
            // valor alcança o salário mínimo vigente. Abaixo disso, fica fora do
            // PBC até a complementação.
            if let Some(minimo) = salario_minimo_vigente(&c) {
                if a < minimo { abaixo_do_minimo.push(c.to_string()); continue; }
            }
            wages.push((c,a));
        }
    }
    if pre94>0 { warnings.push(format!("{pre94} registro(s) pré-07/1994 fora do PBC, sem soma ou conversão monetária histórica; períodos confirmados permanecem separados.")); }
    if !abaixo_do_minimo.is_empty() {
        let lista = abaixo_do_minimo.iter().take(12).cloned().collect::<Vec<_>>().join(", ");
        warnings.push(format!("{} competência(s) com valor abaixo do salário mínimo não entraram no PBC até a complementação: {lista}{}. O CNIS marca esses meses com PREC-MENOR-MIN.", abaixo_do_minimo.len(), if abaixo_do_minimo.len()>12 {", e outras"} else {""}));
    }
    if wages.is_empty() { issue(&mut b,"pbc_missing","competencias","Nenhuma remuneração confirmada válida desde 07/1994."); }
    let o = &r.opcoes;
    for (name,value) in [("corrigirSalarios",o.corrigir_salarios),("descarte",o.descarte),("divisorMinimo",o.divisor_minimo),("aplicarTetoVigenteDer",o.aplicar_teto_vigente_der)] {
        if value.is_none() { issue(&mut b,"option_not_selected",&format!("opcoes.{name}"),"Opção precisa de true/false explícito; não inferir política normativa."); }
    }
    if !o.graca.is_empty() || !o.recolhimentos_atraso.is_empty() { issue(&mut b,"carencia_extras_unsupported","opcoes.graca/recolhimentosAtraso","O modelo nativo soma estas competências sem validação jurídica: operação bloqueada no adaptador."); }
    if nonempty(&o.regra_alvo) { issue(&mut b,"rule_target_unsupported","opcoes.regraAlvo","calcular não aplica seleção de regra-alvo; projeção existe somente na API tipada original."); }
    let reference = if o.referencia_correcao.is_some() { month(&o.referencia_correcao,"opcoes.referenciaCorrecao",&mut b) } else { None };
    if let Some(minimo) = o.descartar_menores_minimo {
        if minimo < 1 || minimo > 600 { issue(&mut b,"descartar_menores_minimo_invalid","opcoes.descartarMenoresMinimo","Mínimo de competências deve ficar entre 1 e 600."); }
        if o.corrigir_salarios != Some(true) { issue(&mut b,"descartar_menores_requires_correction","opcoes.descartarMenoresMinimo","O descarte da EC 103 é definido sobre o PBC corrigido; ative corrigirSalarios."); }
        if o.descarte == Some(true) { issue(&mut b,"descartar_menores_conflict","opcoes.descartarMenoresMinimo","Use o descarte dos menores (EC 103) ou o descarte fixo de 20%, não ambos."); }
    }
    if o.corrigir_salarios==Some(true) && reference.is_none() { issue(&mut b,"correction_reference_missing","opcoes.referenciaCorrecao","Correção exige referência explicitamente informada."); }
    if let (Some(reference),Some(dib))=(reference,dib) {
        if reference != (Competencia {ano:dib.ano as u16,mes:dib.mes}) { issue(&mut b,"correction_reference_mismatch","opcoes.referenciaCorrecao","Referência precisa corresponder ao mês da DIB; não corrigir para data diferente silenciosamente."); }
    }
    if o.corrigir_salarios==Some(true) && o.divisor_minimo==Some(true) { issue(&mut b,"corrected_divisor_unsupported","opcoes.divisorMinimo","PBC corrigido nativo não aplica divisor mínimo: combinação bloqueada, não ignorada."); }
    if o.corrigir_salarios==Some(false) && o.descarte==Some(true) { issue(&mut b,"uncorrected_discard_unsupported","opcoes.descarte","Descarte só existe no caminho corrigido nativo: combinação bloqueada."); }
    if o.corrigir_salarios==Some(true) {
        if let Some(reference)=reference {
            let last = rgps_core::dados::inpc::INPC_MENSAL.last().expect("nonempty INPC").0;
            if reference.to_string().as_str()>last { issue(&mut b,"inpc_out_of_coverage","opcoes.referenciaCorrecao","Sem INPC no snapshot para a referência; não extrapolar nem preencher zero."); }
        }
    } else { warnings.push("Correção desativada explicitamente: média nominal/valores já corrigidos conforme declaração do operador, não RMI normativa validada.".into()); }
    let limit = Decimal::from(1_000_000_000u64);
    let ceiling = o.teto.as_ref().and_then(|_| amount(&o.teto,"opcoes.teto",&mut b,limit));
    let floor = o.piso.as_ref().and_then(|_| amount(&o.piso,"opcoes.piso",&mut b,limit));
    if matches!((ceiling,floor),(Some(t),Some(p)) if p>t) { issue(&mut b,"inverted_limits","opcoes.piso","Piso maior que teto."); }
    if o.aplicar_teto_vigente_der==Some(true) {
        if ceiling.is_some() { issue(&mut b,"conflicting_ceiling","opcoes.teto","Não fornecer teto manual junto ao teto derivado: o nativo ignoraria o manual."); }
        if let Some(d)=dib {
            if d.ano>2026 { issue(&mut b,"ceiling_out_of_coverage","dataReferencia","Teto de anos posteriores ao snapshot não é extrapolado."); }
            if let Some(p)=floor {
                let actual=rgps_core::atualizacao::teto_vigente(&Competencia {ano:d.ano as u16,mes:d.mes}).map_err(|e|e.0)?;
                if p>actual { issue(&mut b,"inverted_limits","opcoes.piso","Piso maior que teto derivado."); }
            }
        }
    }
    if floor.is_none() { warnings.push("Nenhum piso de benefício foi informado; nenhum piso é inventado ou atualizado automaticamente.".into()); }
    let expectation = o.expectativa_sobrevida.as_ref().and_then(|_| amount(&o.expectativa_sobrevida,"opcoes.expectativaSobrevida",&mut b,Decimal::from(150)));
    if expectation.is_some() { warnings.push("Expectativa de sobrevida declarada: o fator usa a fórmula da Lei 9.876/1999 (alíquotas 0,31/0,30, sem piso artificial). Conferir a fonte antes de uso judicial.".into()); }
    if expectation.is_none() { warnings.push("Sem expectativa declarada: o pedágio de 50% usa a tábua de expectativa embutida (vigência mais recente até a DER); conferir com a publicação IBGE.".into()); }
    if !b.is_empty() { return Ok(blocked(b,warnings)); }
    let dib = dib.expect("validated DIB");
    let native = RgpsEntrada {
        nome:None, especie:r.segurado.especie.clone(), sexo:sex, data_nascimento:birth, data_referencia:Some(dib), vinculos:links, competencias:wages,
        opcoes:RgpsOpcoes {corrigir_salarios:o.corrigir_salarios.unwrap(),descarte:o.descarte.unwrap(),divisor_minimo:o.divisor_minimo.unwrap(),descartar_menores_minimo:o.descartar_menores_minimo,referencia_correcao:reference.or(Some(Competencia {ano:dib.ano as u16,mes:dib.mes})),teto:ceiling,piso:floor,aplicar_teto_vigente_der:o.aplicar_teto_vigente_der.unwrap(),expectativa_sobrevida:expectation,periodos_graca:vec![],recolhimentos_atraso:vec![],regra_alvo:None}
    };
    // The copied core is kept byte-identical; overflow/panic never becomes a zero result.
    let result = std::panic::catch_unwind(|| rgps_core::calcular(&native))
        .map_err(|_| "native_engine_failure: calculation aborted; no result produced".to_string())?
        .map_err(|e| format!("native_engine_error: {}",e.0))?;
    Ok(json!({"status":"experimental", "rule_review_status":"unvalidated", "policy_version":POLICY_VERSION,
        "legal_validation_performed":false, "missing":[], "blockers":[],
        "data":{"native_result":result,"pre_1994_wages_excluded":pre94},
        "warnings":warnings,"known_model_risks":model_risks(),"capabilities":capabilities()}))
}
