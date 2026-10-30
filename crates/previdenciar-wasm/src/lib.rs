use rgps_core::calendar::{Competencia, IsoDate};
use rgps_core::engine::{RgpsEntrada, RgpsOpcoes};
use rgps_core::money::parse_money;
use rgps_core::tempo::VinculoEntrada;
use rust_decimal::Decimal;
use serde_json::{json, Value};

fn field<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| v.get(*key))
}

fn text(v: &Value, keys: &[&str]) -> Option<String> {
    field(v, keys).and_then(Value::as_str).map(str::to_owned)
}

fn flag(v: &Value, keys: &[&str]) -> Option<bool> {
    field(v, keys).and_then(Value::as_bool)
}

fn number(v: &Value, keys: &[&str]) -> Option<u64> {
    field(v, keys).and_then(Value::as_u64)
}

fn date(value: &Value, keys: &[&str], label: &str) -> Result<Option<IsoDate>, String> {
    match text(value, keys) {
        Some(raw) if !raw.trim().is_empty() => IsoDate::parse(&raw, label).map(Some).map_err(|e| e.to_string()),
        _ => Ok(None),
    }
}

fn month(value: &Value, keys: &[&str], label: &str) -> Result<Option<Competencia>, String> {
    match text(value, keys) {
        Some(raw) if !raw.trim().is_empty() => Competencia::parse(&raw, label).map(Some).map_err(|e| e.to_string()),
        _ => Ok(None),
    }
}

fn decimal(value: &Value, keys: &[&str], label: &str) -> Result<Option<Decimal>, String> {
    match text(value, keys) {
        Some(raw) if !raw.trim().is_empty() => parse_money(&raw, label).map(Some).map_err(|e| e.to_string()),
        _ => Ok(None),
    }
}

fn options(value: &Value) -> Result<RgpsOpcoes, String> {
    let mut op = RgpsOpcoes::default();
    if let Some(v) = flag(value, &["corrigirSalarios", "corrigir_salarios"]) { op.corrigir_salarios = v; }
    if let Some(v) = flag(value, &["descarte"]) { op.descarte = v; }
    if let Some(v) = flag(value, &["divisorMinimo", "divisor_minimo"]) { op.divisor_minimo = v; }
    if let Some(v) = number(value, &["descartarMenoresMinimo", "descartar_menores_minimo"]) { op.descartar_menores_minimo = Some(v as u32); }
    if let Some(v) = month(value, &["referenciaCorrecao", "referencia_correcao"], "Referência de correção")? { op.referencia_correcao = Some(v); }
    if let Some(v) = decimal(value, &["teto"], "Teto")? { op.teto = Some(v); }
    if let Some(v) = decimal(value, &["piso"], "Piso")? { op.piso = Some(v); }
    if let Some(v) = flag(value, &["aplicarTetoVigenteDer", "aplicar_teto_vigente_der"]) { op.aplicar_teto_vigente_der = v; }
    if let Some(v) = decimal(value, &["expectativaSobrevida", "expectativa_sobrevida"], "Expectativa de sobrevida")? { op.expectativa_sobrevida = Some(v); }
    if let Some(v) = text(value, &["regraAlvo", "regra_alvo"]) { op.regra_alvo = Some(v); }

    if let Some(list) = field(value, &["graca", "periodosGraca", "periodos_graca"]).and_then(Value::as_array) {
        for item in list {
            if let (Some(inicio), Some(fim)) = (date(item, &["inicio"], "Início da graça")?, date(item, &["fim"], "Fim da graça")?) {
                op.periodos_graca.push((inicio, fim));
            }
        }
    }
    if let Some(list) = field(value, &["recolhimentosAtraso", "recolhimentos_atraso"]).and_then(Value::as_array) {
        for item in list {
            let raw = item.as_str().ok_or_else(|| "Recolhimento em atraso precisa ser AAAA-MM.".to_owned())?;
            op.recolhimentos_atraso.push(Competencia::parse(raw, "Recolhimento em atraso").map_err(|e| e.to_string())?);
        }
    }
    Ok(op)
}

fn build(value: &Value) -> Result<RgpsEntrada, String> {
    let insured = field(value, &["segurado"]).ok_or_else(|| "Campo segurado obrigatório.".to_owned())?;
    let sexo = text(insured, &["sexo"]).filter(|s| !s.trim().is_empty()).ok_or_else(|| "Sexo cadastral obrigatório; não inferir pelo nome.".to_owned())?;
    let nome = text(insured, &["nome"]);
    let especie = text(insured, &["especie"]);
    let data_nascimento = date(insured, &["dataNascimento", "data_nascimento"], "Data de nascimento")?;
    let data_referencia = date(value, &["dataReferencia", "data_referencia"], "Data de referência")?;

    let mut vinculos = Vec::new();
    if let Some(list) = field(value, &["vinculos"]).and_then(Value::as_array) {
        for (i, item) in list.iter().enumerate() {
            let inicio = date(item, &["inicio"], "Início do vínculo")?.ok_or_else(|| format!("Vínculo {} sem início.", i + 1))?;
            let fim = date(item, &["fim"], "Fim do vínculo")?;
            let tipo = text(item, &["tipo"]).unwrap_or_else(|| "empregado".to_owned());
            let empresa = text(item, &["empresa"]);
            let fator = item.get("fator").and_then(Value::as_f64).unwrap_or(1.0);
            if !fator.is_finite() || fator <= 0.0 {
                return Err(format!("Fator do vínculo {} precisa ser positivo.", i + 1));
            }
            vinculos.push(VinculoEntrada { inicio, fim, tipo, empresa, fator });
        }
    }

    let mut competencias = Vec::new();
    if let Some(list) = field(value, &["competencias"]).and_then(Value::as_array) {
        for (i, item) in list.iter().enumerate() {
            let comp = month(item, &["competencia", "comp"], "Competência")?.ok_or_else(|| format!("Competência {} sem mês.", i + 1))?;
            let salario = decimal(item, &["salario", "valor", "value"], "Salário")?.ok_or_else(|| format!("Competência {} sem salário.", comp.to_string()))?;
            competencias.push((comp, salario));
        }
    }
    if competencias.is_empty() {
        return Err("Informe ao menos uma competência com salário de contribuição.".to_owned());
    }

    let opcoes = match field(value, &["opcoes"]) {
        Some(op) => options(op)?,
        None => RgpsOpcoes::default(),
    };

    Ok(RgpsEntrada { nome, especie, sexo, data_nascimento, data_referencia, vinculos, competencias, opcoes })
}

pub fn calculate_json(input: &str) -> String {
    let result = (|| -> Result<Value, String> {
        let value: Value = serde_json::from_str(input).map_err(|e| format!("JSON inválido: {e}"))?;
        let entrada = build(&value)?;
        let resultado = rgps_core::calcular(&entrada).map_err(|e| e.to_string())?;
        serde_json::to_value(resultado).map_err(|e| e.to_string())
    })();
    match result {
        Ok(data) => json!({
            "ok": true,
            "engine": "rgps-core",
            "version": rgps_core::ENGINE_VERSION,
            "data": data
        }).to_string(),
        Err(error) => json!({ "ok": false, "engine": "rgps-core", "version": rgps_core::ENGINE_VERSION, "error": error }).to_string(),
    }
}

fn version_json() -> String {
    json!({
        "ok": true,
        "engine": "rgps-core",
        "version": rgps_core::ENGINE_VERSION,
        "wasm": true,
        "first_competence": rgps_core::PRIMEIRA_COMPETENCIA,
        "divisor_minimo": rgps_core::DIVISOR_MINIMO,
        "reform": rgps_core::REFORMA_EC103
    }).to_string()
}

fn pack(value: String) -> *mut u8 {
    let bytes = value.into_bytes();
    let mut buffer = Vec::with_capacity(8 + bytes.len());
    buffer.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    buffer.extend_from_slice(&bytes);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

#[no_mangle]
pub extern "C" fn pio_alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn pio_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        let _ = Vec::from_raw_parts(ptr, len, len);
    }
}

#[no_mangle]
pub unsafe extern "C" fn pio_calculate(ptr: *const u8, len: usize) -> *mut u8 {
    let bytes = std::slice::from_raw_parts(ptr, len);
    let output = match std::str::from_utf8(bytes) {
        Ok(text) => calculate_json(text),
        Err(_) => json!({ "ok": false, "error": "A entrada não é UTF-8 válido." }).to_string(),
    };
    pack(output)
}

#[no_mangle]
pub extern "C" fn pio_version() -> *mut u8 {
    pack(version_json())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_fixture_runs_the_same_core() {
        let input = include_str!("../../previdenciar-rgps/tests/fixtures/synthetic-300-months.json");
        let output: Value = serde_json::from_str(&calculate_json(input)).expect("json output");
        assert_eq!(output["ok"], true, "{output}");
        assert_eq!(output["engine"], "rgps-core");
        assert_eq!(output["data"]["resumo"]["competencias"], 300);
        assert!(output["data"]["resumo"]["rmi"].is_string());
    }

    #[test]
    fn invalid_input_is_an_error_envelope() {
        let output: Value = serde_json::from_str(&calculate_json("{}")).expect("json output");
        assert_eq!(output["ok"], false);
        assert!(output["error"].as_str().unwrap().contains("segurado"));
    }

    #[test]
    fn ec103_discard_is_blocked_in_the_web_engine() {
        let mut input: Value = serde_json::from_str(include_str!("../../previdenciar-rgps/tests/fixtures/synthetic-300-months.json")).unwrap();
        input["opcoes"]["descartarMenoresMinimo"] = json!(240);
        let output: Value = serde_json::from_str(&calculate_json(&input.to_string())).unwrap();
        assert_eq!(output["ok"], false);
        assert!(output["error"].as_str().unwrap().contains("Descarte da EC 103/2019 bloqueado"));
        assert!(output.get("data").is_none());
    }
}
