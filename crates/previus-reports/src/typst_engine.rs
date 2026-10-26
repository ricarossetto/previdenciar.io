//! Renderização dos relatórios por templates Typst, espelhando o layout do
//! produto de referência (logo, campos em caixa, tabelas azul-marinho).
use serde_json::Value as Json;
use typst::foundations::{Dict, IntoValue, Value as TValue};
use typst_as_lib::TypstEngine;

static LOGO: &[u8] = include_bytes!("../assets/logo.png");
static BASE: &str = include_str!("../templates/_base.typ");
static ANALISE: &str = include_str!("../templates/analise.typ");
static TEMPO: &str = include_str!("../templates/tempo.typ");
static TRANSICOES: &str = include_str!("../templates/transicoes.typ");
static RMI: &str = include_str!("../templates/rmi.typ");
static QUALIDADE: &str = include_str!("../templates/qualidade.typ");
static ROI: &str = include_str!("../templates/roi.typ");
static REGRAS: &str = include_str!("../templates/regras.typ");
static GENERICO: &str = include_str!("../templates/generico.typ");

/// Converte JSON em valor Typst, preservando a estrutura de dicionários/arrays.
fn to_typst(v: &Json) -> TValue {
    match v {
        Json::Null => TValue::None,
        Json::Bool(b) => b.into_value(),
        Json::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_value()
            } else {
                n.as_f64().unwrap_or(0.0).into_value()
            }
        }
        Json::String(s) => s.clone().into_value(),
        Json::Array(a) => a.iter().map(to_typst).collect::<Vec<TValue>>().into_value(),
        Json::Object(o) => {
            let mut d = Dict::new();
            for (k, val) in o {
                d.insert(k.as_str().into(), to_typst(val));
            }
            TValue::Dict(d)
        }
    }
}

fn main_source(kind: &str) -> Result<&'static str, String> {
    match kind {
        "analysis" => Ok(ANALISE),
        "periods" => Ok(TEMPO),
        "transitions" => Ok(TRANSICOES),
        "memory" => Ok(RMI),
        "quality" => Ok(QUALIDADE),
        "roi" => Ok(ROI),
        "regras" => Ok(REGRAS),
        "summary" | "salaries" | "indicators" => Ok(GENERICO),
        other => Err(format!("Template Typst ausente para o tipo {other}.")),
    }
}

/// Renderiza um relatório a partir do template Typst e dos dados do caso.
pub fn render(kind: &str, data: &Json) -> Result<Vec<u8>, String> {
    let main = main_source(kind)?;
    let (regular, bold) = crate::layout::report_fonts();
    let template = TypstEngine::builder()
        .main_file(main)
        .fonts([regular.as_slice(), bold.as_slice()])
        .with_static_file_resolver([("logo.png", LOGO)])
        .with_static_source_file_resolver([("_base.typ", BASE.to_string())])
        .build();
    let mut top = Dict::new();
    top.insert("data".into(), to_typst(data));
    let doc = template
        .compile_with_input(top)
        .output
        .map_err(|e| format!("Falha ao compilar o template Typst: {e:?}"))?;
    typst_pdf::pdf(&doc, &Default::default())
        .map_err(|e| format!("Falha ao gerar o PDF Typst: {e:?}"))
}
