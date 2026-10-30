use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

pub const MAX_DOCUMENTOS: usize = 1_000;
pub const MAX_LEITURAS: usize = 5_000;
pub const MAX_FATOS: usize = 20_000;
pub const MAX_TEXTO_EXTRAIDO: usize = 200_000;
pub const MAX_TRECHO: usize = 500;
pub const MAX_CAMPOS: usize = 5_000;
pub const MAX_PENDENCIAS: usize = 500;
pub const MAX_ID: usize = 80;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrigemTexto {
    #[default]
    Nativo,
    Ocr,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoLeitura {
    Cnis,
    LaudoMedico,
    ProcessoAdministrativo,
    Ppp,
    ProvaRural,
    GrupoFamiliar,
    CartaDeConcessao,
    DocumentosDaPensao,
    #[default]
    DocumentoGenerico,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusLeitura {
    #[default]
    EmRevisao,
    Confirmada,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfiancaCampo {
    Alta,
    Media,
    #[default]
    Baixa,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenienciaFato {
    Comprovado,
    #[default]
    Alegado,
    Inferido,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotivoPendencia {
    AusenteNoDocumento,
    Ilegivel,
    Conflitante,
    #[default]
    NaoInformado,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeveridadeAlerta {
    Critico,
    Atencao,
    #[default]
    Info,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Documento {
    pub id: String,
    pub nome_arquivo: String,
    pub mime: String,
    pub paginas: u32,
    pub origem_texto: OrigemTexto,
    pub ocr_confianca: Option<f32>,
    pub hash_sha256: String,
    pub criado_em: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OrigemCampo {
    pub documento_id: String,
    pub pagina: u32,
    pub trecho: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CampoExtraido {
    pub chave: String,
    pub valor: Value,
    pub confianca: ConfiancaCampo,
    pub origem: OrigemCampo,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Pendencia {
    pub campo: String,
    pub motivo: MotivoPendencia,
    pub pergunta: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Leitura {
    pub id: String,
    pub documento_id: String,
    pub tipo: TipoLeitura,
    pub campos: Vec<CampoExtraido>,
    pub paginas_lidas: u32,
    pub pendencias: Vec<Pendencia>,
    pub status: StatusLeitura,
    pub texto_extraido: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Fato {
    pub id: String,
    pub texto: String,
    pub proveniencia: ProvenienciaFato,
    pub origem: OrigemCampo,
    pub alegado_por: Option<String>,
    pub confirmado: bool,
    pub usado_em: Vec<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Alerta {
    pub codigo: String,
    pub severidade: SeveridadeAlerta,
    pub mensagem: String,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max
}

fn valid_scalar(value: &Value) -> bool {
    matches!(value, Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_))
}

fn validate_origin(
    origin: &OrigemCampo,
    documents: &HashSet<&str>,
    pages: &std::collections::HashMap<&str, u32>,
) -> Option<String> {
    if !documents.contains(origin.documento_id.as_str()) {
        return Some("Origem aponta para documento inexistente.".into());
    }
    if origin.pagina == 0 {
        return Some("Página de origem deve ser 1-based.".into());
    }
    if pages
        .get(origin.documento_id.as_str())
        .is_some_and(|total| *total > 0 && origin.pagina > *total)
    {
        return Some("Página de origem excede o total do documento.".into());
    }
    if !valid_text(&origin.trecho, MAX_TRECHO) {
        return Some("Trecho de origem vazio ou acima de 500 caracteres.".into());
    }
    None
}

/// Structural and referential validation for the evidence layer. It never
/// evaluates the truth of a fact or the merits of a calculation.
pub fn validate(
    documents: &[Documento],
    readings: &[Leitura],
    facts: &[Fato],
    calculation_ids: &HashSet<u64>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if documents.len() > MAX_DOCUMENTOS
        || readings.len() > MAX_LEITURAS
        || facts.len() > MAX_FATOS
    {
        warnings.push("Quantidade de documentos, leituras ou fatos excede o limite local.".into());
        return warnings;
    }
    let mut document_ids = HashSet::new();
    let mut pages = std::collections::HashMap::new();
    for document in documents {
        if !valid_id(&document.id) || !document_ids.insert(document.id.as_str()) {
            warnings.push("Documento com ID vazio, inválido ou duplicado.".into());
            continue;
        }
        if document.nome_arquivo.trim().is_empty() || document.nome_arquivo.chars().count() > 260 {
            warnings.push("Documento com nome de arquivo vazio ou acima de 260 caracteres.".into());
        }
        if document.mime.trim().is_empty() || document.mime.chars().count() > 120 {
            warnings.push("Documento com MIME vazio ou inválido.".into());
        }
        if document.paginas > 500 {
            warnings.push("Documento excede o limite de 500 páginas.".into());
        }
        if !valid_hash(&document.hash_sha256) {
            warnings.push("Documento exige hash SHA-256 com 64 dígitos hexadecimais.".into());
        }
        if document.ocr_confianca.is_some_and(|value| !(0.0..=1.0).contains(&value)) {
            warnings.push("Confiança de OCR deve ficar entre 0 e 1.".into());
        }
        if document.criado_em.trim().is_empty() || document.criado_em.chars().count() > 40 {
            warnings.push("Data de criação do documento ausente ou inválida.".into());
        }
        pages.insert(document.id.as_str(), document.paginas);
    }

    let mut reading_ids = HashSet::new();
    for reading in readings {
        if !valid_id(&reading.id) || !reading_ids.insert(reading.id.as_str()) {
            warnings.push("Leitura com ID vazio, inválido ou duplicado.".into());
            continue;
        }
        if !document_ids.contains(reading.documento_id.as_str()) {
            warnings.push("Leitura aponta para documento inexistente.".into());
        }
        if reading.paginas_lidas > 500
            || pages
                .get(reading.documento_id.as_str())
                .is_some_and(|total| *total > 0 && reading.paginas_lidas > *total)
        {
            warnings.push("Páginas lidas da leitura excedem o documento ou o limite local.".into());
        }
        if reading.campos.len() > MAX_CAMPOS {
            warnings.push("Leitura excede o limite de campos extraídos.".into());
        }
        if reading.pendencias.len() > MAX_PENDENCIAS {
            warnings.push("Leitura excede o limite de pendências.".into());
        }
        if reading
            .texto_extraido
            .as_ref()
            .is_some_and(|text| text.chars().count() > MAX_TEXTO_EXTRAIDO)
        {
            warnings.push("Texto extraído excede 200 mil caracteres.".into());
        }
        let mut keys = HashSet::new();
        for field in &reading.campos {
            if !valid_text(&field.chave, 160) || !keys.insert(field.chave.as_str()) {
                warnings.push("Campo extraído com chave vazia, longa ou duplicada.".into());
            }
            if !valid_scalar(&field.valor) {
                warnings.push("Campo extraído deve conter valor escalar ou nulo.".into());
            }
            if let Some(message) = validate_origin(&field.origem, &document_ids, &pages) {
                warnings.push(format!("Campo extraído: {message}"));
            } else if field.origem.documento_id != reading.documento_id {
                warnings.push("Campo extraído aponta para documento diferente da leitura.".into());
            }
        }
        for pending in &reading.pendencias {
            if pending.campo.trim().is_empty() || pending.campo.chars().count() > 160 {
                warnings.push("Pendência com campo vazio ou acima de 160 caracteres.".into());
            }
            if pending.pergunta.trim().is_empty() || pending.pergunta.chars().count() > 500 {
                warnings.push("Pendência com pergunta vazia ou acima de 500 caracteres.".into());
            }
        }
    }

    let mut fact_ids = HashSet::new();
    for fact in facts {
        if !valid_id(&fact.id) || !fact_ids.insert(fact.id.as_str()) {
            warnings.push("Fato com ID vazio, inválido ou duplicado.".into());
            continue;
        }
        if !valid_text(&fact.texto, 2_000) {
            warnings.push("Fato com texto vazio ou acima de 2 mil caracteres.".into());
        }
        if let Some(message) = validate_origin(&fact.origem, &document_ids, &pages) {
            warnings.push(format!("Fato: {message}"));
        }
        if fact.proveniencia == ProvenienciaFato::Alegado
            && fact.alegado_por.as_ref().is_none_or(|value| !valid_text(value, 160))
        {
            warnings.push("Fato alegado exige a indicação de quem alega.".into());
        }
        for id in &fact.usado_em {
            if !calculation_ids.contains(id) {
                warnings.push("Fato aponta para cálculo inexistente.".into());
            }
        }
    }

    warnings.sort();
    warnings.dedup();
    warnings
}
