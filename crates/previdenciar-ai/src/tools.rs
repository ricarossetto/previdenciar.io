use crate::error::{AiError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

const MAX_INDEX_BYTES: u64 = 12 * 1024 * 1024;
const MAX_READ_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TOOL_CHARS: usize = 9_000;

#[derive(Clone)]
pub struct DataStore {
    root: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStatus {
    pub skills: usize,
    pub legislation: usize,
    pub natjus: usize,
    pub natjus_text: usize,
    pub jurisprudence: usize,
    pub ready: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub label: String,
    pub title: String,
    pub url: Option<String>,
    pub kind: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeBlock {
    Skills,
    Legislation,
    Natjus,
    Jurisprudence,
}

impl KnowledgeBlock {
    fn search_tool(self) -> &'static str {
        match self {
            Self::Skills => "buscar_habilidades",
            Self::Legislation => "buscar_legislacao",
            Self::Natjus => "buscar_natjus",
            Self::Jurisprudence => "buscar_jurisprudencia",
        }
    }

    fn read_tool(self) -> &'static str {
        match self {
            Self::Skills => "ler_habilidade",
            Self::Legislation => "ler_legislacao",
            Self::Natjus => "ler_natjus",
            Self::Jurisprudence => "ler_jurisprudencia",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolOutcome {
    pub summary: String,
    pub sources: Vec<SourceRef>,
    pub value: Value,
}

struct SearchDoc {
    key: String,
    title: String,
    subtitle: String,
    url: Option<String>,
    tokens: Vec<String>,
    payload: Value,
}

impl DataStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn status(&self) -> DataStatus {
        let mut errors = Vec::new();
        let skills = self.count_or(&mut errors, "skills/index.json");
        let legislation = self.count_or(&mut errors, "legislation/index.json");
        let natjus = self.count_or(&mut errors, "natjus/revisoes.json");
        let natjus_text = self.count_text_files("natjus/pdf-text");
        let jurisprudence = self.count_or(&mut errors, "jurisprudence.json");
        let total = skills + legislation + natjus + jurisprudence;
        let mut message = if total == 0 {
            "Base de conhecimento local não importada. Use scripts/import-base-conhecimento.ps1 para copiar os índices e textos para %LOCALAPPDATA%\\Previdenciar\\conhecimento.".to_owned()
        } else if errors.is_empty() {
            format!("Base local pronta: {skills} habilidades, {legislation} normas, {natjus} NatJus ({natjus_text} com texto de PDF) e {jurisprudence} ementas.")
        } else {
            format!(
                "Base local parcial; {total} registros lidos. {}",
                errors.join(" ")
            )
        };
        if !errors.is_empty() && total > 0 {
            message.push_str(" Reimporte os índices para corrigir arquivos inválidos.");
        }
        DataStatus {
            skills,
            legislation,
            natjus,
            natjus_text,
            jurisprudence,
            ready: total > 0,
            message,
        }
    }

    fn count_text_files(&self, relative: &str) -> usize {
        let path = match self.safe_join(relative) {
            Ok(path) => path,
            Err(_) => return 0,
        };
        fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry.path().extension().and_then(|value| value.to_str()) == Some("txt")
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    fn count_or(&self, errors: &mut Vec<String>, relative: &str) -> usize {
        match self.read_array(relative) {
            Ok(items) => items.len(),
            Err(error) => {
                errors.push(error.to_string());
                0
            }
        }
    }

    fn safe_join(&self, relative: &str) -> Result<PathBuf> {
        if relative.trim().is_empty() || relative.contains('\0') {
            return Err(AiError::Data("Caminho de dados vazio ou inválido.".into()));
        }
        let mut candidate = self.root.clone();
        let path = Path::new(relative);
        if path.is_absolute() {
            return Err(AiError::Data(
                "Caminho absoluto recusado na base local.".into(),
            ));
        }
        for component in path.components() {
            match component {
                Component::Normal(value) => candidate.push(value),
                _ => {
                    return Err(AiError::Data(
                        "Travessia de diretório recusada na base local.".into(),
                    ))
                }
            }
        }
        let root = self
            .root
            .canonicalize()
            .map_err(|_| AiError::Data("A base de conhecimento local ainda não foi importada.".into()))?;
        let canonical = candidate
            .canonicalize()
            .map_err(|_| AiError::Data("Arquivo da base local não encontrado.".into()))?;
        if !canonical.starts_with(&root) {
            return Err(AiError::Data("Arquivo fora da base local recusado.".into()));
        }
        Ok(canonical)
    }

    fn read_text(&self, relative: &str) -> Result<String> {
        let path = self.safe_join(relative)?;
        let meta = fs::symlink_metadata(&path).map_err(|_| {
            AiError::Data("Não foi possível inspecionar o arquivo da base local.".into())
        })?;
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(AiError::Data("Arquivo da base local não e regular.".into()));
        }
        if meta.len() > MAX_READ_BYTES {
            return Err(AiError::Data(
                "Arquivo da base local excede o limite de leitura.".into(),
            ));
        }
        fs::read_to_string(&path)
            .map_err(|_| AiError::Data("Não foi possível ler o arquivo da base local.".into()))
    }

    fn read_array(&self, relative: &str) -> Result<Vec<Value>> {
        let path = self.safe_join(relative)?;
        let meta = fs::symlink_metadata(&path)
            .map_err(|_| AiError::Data("Não foi possível inspecionar o índice local.".into()))?;
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(AiError::Data(
                "Indice local não e um arquivo regular.".into(),
            ));
        }
        if meta.len() > MAX_INDEX_BYTES {
            return Err(AiError::Data(
                "Indice local excede o limite de leitura.".into(),
            ));
        }
        let bytes = fs::read(&path)
            .map_err(|_| AiError::Data("Não foi possível ler o índice local.".into()))?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| AiError::Data(format!("Índice local inválido: {relative}.")))?;
        match value {
            Value::Array(items) => Ok(items),
            _ => Err(AiError::Data(format!(
                "Indice local não e uma lista: {relative}."
            ))),
        }
    }

    pub fn tools() -> Vec<ToolDef> {
        let search = |name: &str, description: &str, noun: &str| ToolDef {
            name: name.into(),
            description: description.into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "consulta": {"type": "string", "description": format!("Termos de busca sobre {noun}.")},
                    "limite": {"type": "integer", "minimum": 1, "maximum": 8, "description": "Maximo de resultados."}
                },
                "required": ["consulta"],
                "additionalProperties": false
            }),
        };
        let read = |name: &str, description: &str| ToolDef {
            name: name.into(),
            description: description.into(),
            parameters: json!({
                "type": "object",
                "properties": {"id": {"type": "string", "description": "Identificador retornado pela busca."}},
                "required": ["id"],
                "additionalProperties": false
            }),
        };
        vec![
            search("buscar_habilidades", "Pesquisa o catalogo local de habilidades da base de conhecimento (metodos, estruturas e estilos de peca).", "habilidades juridicas"),
            read("ler_habilidade", "Lê o conteúdo completo de uma habilidade pelo id retornado na busca."),
            search("buscar_legislacao", "Pesquisa o boletim local de legislacao federal importado da base de conhecimento.", "legislacao federal"),
            read("ler_legislacao", "Le a ficha local de uma norma pelo id retornado na busca."),
            search("buscar_natjus", "Pesquisa o catalogo local de pareceres tecnicos e-NatJus por intervencao ou tecnologia em saude.", "NatJus"),
            read("ler_natjus", "Le os metadados locais de um parecer NatJus pelo id retornado na busca."),
            search("buscar_jurisprudencia", "Pesquisa a base local de ementas importadas, com tribunal, processo e link oficial.", "jurisprudencia"),
            read("ler_jurisprudencia", "Le a ementa completa de um precedente pelo id retornado na busca."),
        ]
    }

    pub fn search_block(
        &self,
        block: KnowledgeBlock,
        query: &str,
        limit: usize,
    ) -> Result<ToolOutcome> {
        self.execute(
            block.search_tool(),
            &json!({"consulta": query, "limite": limit}),
        )
    }

    pub fn read_block(&self, block: KnowledgeBlock, id: &str) -> Result<ToolOutcome> {
        self.execute(block.read_tool(), &json!({"id": id}))
    }

    pub fn execute(&self, name: &str, arguments: &Value) -> Result<ToolOutcome> {
        let limit = argument_limit(arguments);
        let id = || argument_string(arguments, "id");
        match name {
            "buscar_habilidades" => {
                self.search_skills(argument_string(arguments, "consulta")?, limit)
            }
            "ler_habilidade" => self.read_skill(&id()?),
            "buscar_legislacao" => {
                self.search_legislation(argument_string(arguments, "consulta")?, limit)
            }
            "ler_legislacao" => self.read_legislation(&id()?),
            "buscar_natjus" => self.search_natjus(argument_string(arguments, "consulta")?, limit),
            "ler_natjus" => self.read_natjus(&id()?),
            "buscar_jurisprudencia" => {
                self.search_jurisprudence(argument_string(arguments, "consulta")?, limit)
            }
            "ler_jurisprudencia" => self.read_jurisprudence(&id()?),
            _ => Err(AiError::Invalid(format!(
                "Ferramenta local desconhecida: {name}."
            ))),
        }
    }

    fn skill_docs(&self) -> Result<Vec<SearchDoc>> {
        let entries = self.read_array("skills/index.json")?;
        let mut docs = Vec::with_capacity(entries.len());
        for entry in entries {
            let Some(stable_id) = string_field(&entry, "stableId") else {
                continue;
            };
            let Some(title) = string_field(&entry, "title") else {
                continue;
            };
            let Some(md_path) = string_field(&entry, "md_path") else {
                continue;
            };
            if md_path.contains("..") || md_path.contains(':') {
                continue;
            }
            let materia = string_field(&entry, "materia").unwrap_or_default();
            let document_type = string_field(&entry, "documentType").unwrap_or_default();
            let triggers = entry
                .get("triggers")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            let subtitle = [materia.as_str(), document_type.as_str()]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" / ");
            let searchable = format!("{title} {title} {triggers} {triggers} {subtitle}");
            docs.push(SearchDoc {
                key: stable_id.clone(),
                title: title.clone(),
                subtitle,
                url: None,
                tokens: tokens(&searchable),
                payload: json!({
                    "id": stable_id,
                    "titulo": title,
                    "materia": materia,
                    "tipo_documento": document_type,
                    "gatilhos": entry.get("triggers").cloned().unwrap_or_else(|| json!([]))
                }),
            });
        }
        Ok(docs)
    }

    fn search_skills(&self, query: &str, limit: usize) -> Result<ToolOutcome> {
        let docs = self.skill_docs()?;
        let ranked = rank(&docs, query, limit);
        let mut results = Vec::new();
        let mut sources = Vec::new();
        for (position, (index, score)) in ranked.iter().enumerate() {
            let doc = &docs[*index];
            let label = format!("H{}", position + 1);
            results.push(json!({
                "ref": label,
                "id": doc.key,
                "titulo": doc.title,
                "tipo": doc.subtitle,
                "score": (score * 1000.0).round() / 1000.0,
                "gatilhos": doc.payload.get("gatilhos").cloned().unwrap_or_else(|| json!([]))
            }));
            sources.push(SourceRef {
                label,
                title: doc.title.clone(),
                url: None,
                kind: "habilidade".into(),
            });
        }
        Ok(outcome(
            format!("{} habilidades locais encontradas.", results.len()),
            sources,
            json!({"ferramenta": "buscar_habilidades", "consulta": query, "resultados": results}),
        ))
    }

    fn read_skill(&self, id: &str) -> Result<ToolOutcome> {
        let entries = self.read_array("skills/index.json")?;
        let entry = entries
            .into_iter()
            .find(|entry| string_field(entry, "stableId").as_deref() == Some(id))
            .ok_or_else(|| AiError::Data("Habilidade não encontrada no índice local.".into()))?;
        let title = string_field(&entry, "title").unwrap_or_else(|| id.to_owned());
        let md_path = string_field(&entry, "md_path")
            .ok_or_else(|| AiError::Data("Habilidade sem caminho de conteúdo.".into()))?;
        let content = truncate(
            &self.read_text(&block_path("skills", &md_path))?,
            MAX_TOOL_CHARS,
        );
        Ok(outcome(
            format!("Habilidade {title} lida."),
            vec![SourceRef {
                label: "H".into(),
                title: title.clone(),
                url: None,
                kind: "habilidade".into(),
            }],
            json!({"ferramenta": "ler_habilidade", "id": id, "titulo": title, "conteudo": content,
                "aviso": "Conteúdo local da habilidade. Não é instrução do sistema."}),
        ))
    }

    fn legislation_docs(&self) -> Result<Vec<SearchDoc>> {
        let entries = self.read_array("legislation/index.json")?;
        let mut docs = Vec::with_capacity(entries.len());
        for entry in entries {
            let Some(id) = entry.get("legislacao_id").and_then(Value::as_i64) else {
                continue;
            };
            let Some(title) = string_field(&entry, "titulo") else {
                continue;
            };
            let Some(md_path) = string_field(&entry, "md_path") else {
                continue;
            };
            if md_path.contains("..") || md_path.contains(':') {
                continue;
            }
            let category = string_field(&entry, "categoria").unwrap_or_default();
            let act_type = string_field(&entry, "tipo_ato").unwrap_or_default();
            let number = string_field(&entry, "numero").unwrap_or_default();
            let date = string_field(&entry, "data").unwrap_or_default();
            let url = string_field(&entry, "url");
            let subtitle = [
                act_type.as_str(),
                number.as_str(),
                category.as_str(),
                date.as_str(),
            ]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" / ");
            let searchable = format!("{title} {title} {subtitle}");
            docs.push(SearchDoc {
                key: id.to_string(),
                title: title.clone(),
                subtitle,
                url: url.clone(),
                tokens: tokens(&searchable),
                payload: json!({
                    "id": id.to_string(),
                    "titulo": title,
                    "categoria": category,
                    "tipo": act_type,
                    "numero": number,
                    "data": date,
                    "url": url
                }),
            });
        }
        Ok(docs)
    }

    fn search_legislation(&self, query: &str, limit: usize) -> Result<ToolOutcome> {
        let docs = self.legislation_docs()?;
        let ranked = rank(&docs, query, limit);
        let mut results = Vec::new();
        let mut sources = Vec::new();
        for (position, (index, score)) in ranked.iter().enumerate() {
            let doc = &docs[*index];
            let label = format!("L{}", position + 1);
            results.push(json!({
                "ref": label,
                "id": doc.key,
                "titulo": doc.title,
                "tipo": doc.subtitle,
                "url": doc.url,
                "score": (score * 1000.0).round() / 1000.0
            }));
            sources.push(SourceRef {
                label,
                title: doc.title.clone(),
                url: doc.url.clone(),
                kind: "legislacao".into(),
            });
        }
        Ok(outcome(
            format!("{} normas locais encontradas.", results.len()),
            sources,
            json!({"ferramenta": "buscar_legislacao", "consulta": query, "resultados": results}),
        ))
    }

    fn read_legislation(&self, id: &str) -> Result<ToolOutcome> {
        let entries = self.read_array("legislation/index.json")?;
        let entry = entries
            .into_iter()
            .find(|entry| {
                entry
                    .get("legislacao_id")
                    .and_then(Value::as_i64)
                    .map(|value| value.to_string())
                    .as_deref()
                    == Some(id)
            })
            .ok_or_else(|| AiError::Data("Norma não encontrada no índice local.".into()))?;
        let title = string_field(&entry, "titulo").unwrap_or_else(|| id.to_owned());
        let md_path = string_field(&entry, "md_path")
            .ok_or_else(|| AiError::Data("Norma sem caminho de conteúdo.".into()))?;
        let url = string_field(&entry, "url");
        let content = truncate(
            &self.read_text(&block_path("legislation", &md_path))?,
            MAX_TOOL_CHARS,
        );
        Ok(outcome(
            format!("Norma {title} lida."),
            vec![SourceRef {
                label: "L".into(),
                title: title.clone(),
                url: url.clone(),
                kind: "legislacao".into(),
            }],
            json!({"ferramenta": "ler_legislacao", "id": id, "titulo": title, "url": url, "conteudo": content,
                "aviso": "Ficha local importada; confira a redacao oficial antes de citar."}),
        ))
    }

    fn natjus_docs(&self) -> Result<Vec<SearchDoc>> {
        let entries = self.read_array("natjus/revisoes.json")?;
        let mut docs = Vec::with_capacity(entries.len());
        for entry in entries {
            let Some(id) = entry.get("idParecerTecnico").and_then(Value::as_i64) else {
                continue;
            };
            let Some(title) = string_field(&entry, "intervencao") else {
                continue;
            };
            let Some(md_path) = string_field(&entry, "md_path") else {
                continue;
            };
            if md_path.contains("..") || md_path.contains(':') {
                continue;
            }
            let status = string_field(&entry, "status").unwrap_or_default();
            let date = string_field(&entry, "dataCriacao").unwrap_or_default();
            let url = string_field(&entry, "pdf_url");
            let subtitle = [status.as_str(), date.as_str()]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" / ");
            let text = self
                .read_text(&format!("natjus/pdf-text/{id}.txt"))
                .ok()
                .map(|value| value.chars().take(50_000).collect::<String>())
                .filter(|value| !value.trim().is_empty());
            let searchable = format!("{title} {title} {}", text.as_deref().unwrap_or_default());
            docs.push(SearchDoc {
                key: id.to_string(),
                title: title.clone(),
                subtitle,
                url: url.clone(),
                tokens: tokens(&searchable),
                payload: json!({
                    "id": id.to_string(),
                    "titulo": title,
                    "status": status,
                    "data": date,
                    "url": url,
                    "texto_disponível": text.is_some(),
                    "trecho": text.as_deref().map(|value| truncate(value, 500))
                }),
            });
        }
        Ok(docs)
    }

    fn search_natjus(&self, query: &str, limit: usize) -> Result<ToolOutcome> {
        let docs = self.natjus_docs()?;
        let ranked = rank(&docs, query, limit);
        let mut results = Vec::new();
        let mut sources = Vec::new();
        for (position, (index, score)) in ranked.iter().enumerate() {
            let doc = &docs[*index];
            let label = format!("N{}", position + 1);
            results.push(json!({
                "ref": label,
                "id": doc.key,
                "titulo": doc.title,
                "status": doc.subtitle,
                "url": doc.url,
                "texto_disponível": doc.payload.get("texto_disponível").cloned().unwrap_or(json!(false)),
                "trecho": doc.payload.get("trecho").cloned().unwrap_or(Value::Null),
                "score": (score * 1000.0).round() / 1000.0
            }));
            sources.push(SourceRef {
                label,
                title: doc.title.clone(),
                url: doc.url.clone(),
                kind: "natjus".into(),
            });
        }
        Ok(outcome(
            format!("{} pareceres NatJus encontrados.", results.len()),
            sources,
            json!({"ferramenta": "buscar_natjus", "consulta": query, "resultados": results,
                "aviso": "O índice local contem metadados e texto extraído do PDF quando disponível; confira o documento oficial no e-NatJus."}),
        ))
    }

    fn read_natjus(&self, id: &str) -> Result<ToolOutcome> {
        let entries = self.read_array("natjus/revisoes.json")?;
        let entry = entries
            .into_iter()
            .find(|entry| {
                entry
                    .get("idParecerTecnico")
                    .and_then(Value::as_i64)
                    .map(|value| value.to_string())
                    .as_deref()
                    == Some(id)
            })
            .ok_or_else(|| {
                AiError::Data("Parecer NatJus não encontrado no índice local.".into())
            })?;
        let title = string_field(&entry, "intervencao").unwrap_or_else(|| id.to_owned());
        let md_path = string_field(&entry, "md_path")
            .ok_or_else(|| AiError::Data("Parecer NatJus sem caminho de conteúdo.".into()))?;
        let url = string_field(&entry, "pdf_url");
        let pdf_text = self.read_text(&format!("natjus/pdf-text/{id}.txt")).ok();
        let (content, source_kind) = match pdf_text {
            Some(text) => (truncate(&text, MAX_TOOL_CHARS), "pdf"),
            None => (
                truncate(
                    &self.read_text(&block_path("natjus", &md_path))?,
                    MAX_TOOL_CHARS,
                ),
                "ficha",
            ),
        };
        Ok(outcome(
            format!("Parecer NatJus {title} lido."),
            vec![SourceRef {
                label: "N".into(),
                title: title.clone(),
                url: url.clone(),
                kind: "natjus".into(),
            }],
            json!({"ferramenta": "ler_natjus", "id": id, "titulo": title, "url": url, "conteudo": content,
                "fonte": source_kind,
                "aviso": "Texto local extraído do PDF ou da ficha; o documento oficial e a avaliacao medica devem ser conferidos no e-NatJus."}),
        ))
    }

    fn jurisprudence_docs(&self) -> Result<Vec<SearchDoc>> {
        let entries = self.read_array("jurisprudence.json")?;
        let mut docs = Vec::with_capacity(entries.len());
        for entry in entries {
            let Some(title) = string_field(&entry, "titulo") else {
                continue;
            };
            let Some(ementa) = string_field(&entry, "ementa") else {
                continue;
            };
            let key = string_field(&entry, "key").unwrap_or_else(|| {
                let tribunal = string_field(&entry, "tribunal").unwrap_or_default();
                let id = string_field(&entry, "id").unwrap_or_default();
                format!("{tribunal}-{id}")
            });
            let tribunal = string_field(&entry, "tribunal").unwrap_or_default();
            let relator = string_field(&entry, "relator").unwrap_or_default();
            let orgao = string_field(&entry, "orgao").unwrap_or_default();
            let data = string_field(&entry, "data").unwrap_or_default();
            let processo = string_field(&entry, "processo").unwrap_or_default();
            let url = string_field(&entry, "url");
            let subtitle = [
                tribunal.as_str(),
                processo.as_str(),
                relator.as_str(),
                data.as_str(),
            ]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" / ");
            docs.push(SearchDoc {
                key: key.clone(),
                title: title.clone(),
                subtitle: subtitle.clone(),
                url: url.clone(),
                tokens: tokens(&format!("{title} {ementa} {ementa} {subtitle}")),
                payload: json!({
                    "id": key,
                    "titulo": title,
                    "tribunal": tribunal,
                    "processo": processo,
                    "relator": relator,
                    "orgao": orgao,
                    "data": data,
                    "ementa": ementa,
                    "url": url
                }),
            });
        }
        Ok(docs)
    }

    fn search_jurisprudence(&self, query: &str, limit: usize) -> Result<ToolOutcome> {
        let docs = self.jurisprudence_docs()?;
        let ranked = rank(&docs, query, limit);
        let mut results = Vec::new();
        let mut sources = Vec::new();
        for (position, (index, score)) in ranked.iter().enumerate() {
            let doc = &docs[*index];
            let label = format!("J{}", position + 1);
            results.push(json!({
                "ref": label,
                "id": doc.key,
                "titulo": doc.title,
                "identificacao": doc.subtitle,
                "url": doc.url,
                "trecho": truncate(
                    doc.payload.get("ementa").and_then(Value::as_str).unwrap_or_default(),
                    700
                ),
                "score": (score * 1000.0).round() / 1000.0
            }));
            sources.push(SourceRef {
                label,
                title: doc.title.clone(),
                url: doc.url.clone(),
                kind: "jurisprudencia".into(),
            });
        }
        Ok(outcome(
            format!("{} ementas locais encontradas.", results.len()),
            sources,
            json!({"ferramenta": "buscar_jurisprudencia", "consulta": query, "resultados": results,
                "aviso": "A base local e uma amostra importada. Confira o inteiro teor e o link oficial."}),
        ))
    }

    fn read_jurisprudence(&self, id: &str) -> Result<ToolOutcome> {
        let entries = self.read_array("jurisprudence.json")?;
        let entry = entries
            .into_iter()
            .find(|entry| {
                string_field(entry, "key").as_deref() == Some(id)
                    || format!(
                        "{}-{}",
                        string_field(entry, "tribunal").unwrap_or_default(),
                        string_field(entry, "id").unwrap_or_default()
                    ) == id
            })
            .ok_or_else(|| AiError::Data("Ementa não encontrada na base local.".into()))?;
        let title = string_field(&entry, "titulo").unwrap_or_else(|| id.to_owned());
        let ementa = string_field(&entry, "ementa")
            .ok_or_else(|| AiError::Data("Ementa sem texto no índice local.".into()))?;
        let url = string_field(&entry, "url");
        Ok(outcome(
            format!("Ementa {title} lida."),
            vec![SourceRef {
                label: "J".into(),
                title: title.clone(),
                url: url.clone(),
                kind: "jurisprudencia".into(),
            }],
            json!({"ferramenta": "ler_jurisprudencia", "id": id, "titulo": title, "url": url,
                "ementa": truncate(&ementa, MAX_TOOL_CHARS),
                "aviso": "Ementa importada; confira o inteiro teor antes de citar."}),
        ))
    }
}

fn block_path(block: &str, relative: &str) -> String {
    let relative = relative.trim_start_matches('/').trim();
    let prefix = format!("{block}/");
    if relative.starts_with(&prefix) {
        relative.to_owned()
    } else {
        format!("{prefix}{relative}")
    }
}

fn outcome(summary: String, sources: Vec<SourceRef>, value: Value) -> ToolOutcome {
    ToolOutcome {
        summary,
        sources,
        value,
    }
}

fn argument_string<'a>(arguments: &'a Value, name: &str) -> Result<&'a str> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AiError::Invalid(format!("Argumento obrigatorio ausente: {name}.")))
}

fn argument_limit(arguments: &Value) -> usize {
    arguments
        .get("limite")
        .and_then(Value::as_u64)
        .map(|value| value.clamp(1, 8) as usize)
        .unwrap_or(5)
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_owned();
    }
    let mut output: String = value.chars().take(limit).collect();
    output.push_str("\n\n[conteúdo truncado; leia o arquivo local para o restante]");
    output
}

fn normalize(value: &str) -> String {
    value
        .nfd()
        .filter(|character| !is_combining_mark(*character))
        .collect::<String>()
        .to_lowercase()
}

fn tokens(value: &str) -> Vec<String> {
    normalize(value)
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| token.len() >= 2)
        .filter(|token| !STOPWORDS.contains(token))
        .map(str::to_owned)
        .collect()
}

fn term_counts(tokens: &[String]) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for token in tokens {
        *counts.entry(token.as_str()).or_insert(0) += 1;
    }
    counts
}

fn rank(docs: &[SearchDoc], query: &str, limit: usize) -> Vec<(usize, f64)> {
    let query_terms = tokens(query);
    if docs.is_empty() || query_terms.is_empty() {
        return Vec::new();
    }
    let unique_terms: Vec<&str> = {
        let mut seen = HashSet::new();
        query_terms
            .iter()
            .map(String::as_str)
            .filter(|term| seen.insert(*term))
            .collect()
    };
    let total = docs.len() as f64;
    let average = docs.iter().map(|doc| doc.tokens.len()).sum::<usize>() as f64 / total.max(1.0);
    let mut document_frequency = HashMap::<&str, usize>::new();
    for term in &unique_terms {
        let count = docs
            .iter()
            .filter(|doc| doc.tokens.iter().any(|token| token == term))
            .count();
        document_frequency.insert(term, count);
    }
    let k1 = 1.4;
    let b = 0.72;
    let mut ranked = Vec::new();
    for (index, doc) in docs.iter().enumerate() {
        let counts = term_counts(&doc.tokens);
        let length = doc.tokens.len() as f64;
        let mut score = 0.0;
        for term in &unique_terms {
            let frequency = *counts.get(term).unwrap_or(&0) as f64;
            if frequency == 0.0 {
                continue;
            }
            let df = *document_frequency.get(term).unwrap_or(&1) as f64;
            let idf = (1.0 + (total - df + 0.5) / (df + 0.5)).ln();
            score += idf * (frequency * (k1 + 1.0))
                / (frequency + k1 * (1.0 - b + b * length / average.max(1.0)));
        }
        if score > 0.0 {
            ranked.push((index, score));
        }
    }
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| docs[left.0].title.cmp(&docs[right.0].title))
    });
    ranked.truncate(limit.clamp(1, 8));
    ranked
}

const STOPWORDS: &[&str] = &[
    "a", "o", "as", "os", "um", "uma", "uns", "umas", "de", "da", "do", "das", "dos", "em", "na",
    "no", "nas", "nos", "por", "para", "com", "sem", "que", "se", "como", "qual", "quais", "ao",
    "aos", "e", "ou", "the", "of", "and", "to", "in", "on", "for", "is", "are",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> (tempfile::TempDir, DataStore) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("skills/md")).unwrap();
        fs::create_dir_all(dir.path().join("legislation/normas")).unwrap();
        fs::create_dir_all(dir.path().join("natjus/revisoes")).unwrap();
        fs::write(
            dir.path().join("skills/index.json"),
            r#"[{"stableId":"calculo-rmi","title":"Calculo de RMI","materia":"previdenciario","documentType":"calculo","triggers":["RMI","salario de beneficio"],"md_path":"md/calculo-rmi.md"}]"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("skills/md/calculo-rmi.md"),
            "# RMI\n\nConteudo sintetico.",
        )
        .unwrap();
        fs::write(
            dir.path().join("legislation/index.json"),
            r#"[{"legislacao_id":1,"titulo":"Lei 8.213/1991","categoria":"previdenciario","tipo_ato":"Lei","numero":"8.213","data":"1991-07-24","url":"https://example.test/lei","md_path":"normas/8213.md"}]"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("legislation/normas/8213.md"),
            "# Lei 8.213\n\nTexto sintetico.",
        )
        .unwrap();
        fs::write(
            dir.path().join("natjus/revisoes.json"),
            r#"[{"idParecerTecnico":7,"intervencao":"Medicamento sintetico","status":"Finalizado","dataCriacao":"2026-01-01","pdf_url":"https://example.test/natjus","md_path":"revisoes/7.md"}]"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("natjus/revisoes/7.md"),
            "# NatJus\n\nMetadados sinteticos.",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("natjus/pdf-text")).unwrap();
        fs::write(
            dir.path().join("natjus/pdf-text/7.txt"),
            "Texto extraído do PDF sintetico.",
        )
        .unwrap();
        fs::write(
            dir.path().join("jurisprudence.json"),
            r#"[{"key":"stj-1","id":"1","tribunal":"stj","titulo":"Precedente sintetico","ementa":"Beneficio previdenciario e coisa julgada.","relator":"Relator","orgao":"Turma","data":"2026-01-01","processo":"REsp 1","url":"https://example.test/j"}]"#,
        )
        .unwrap();
        let store = DataStore::new(dir.path().to_path_buf());
        (dir, store)
    }

    #[test]
    fn status_and_search_use_local_indexes() {
        let (_dir, store) = fixture();
        let status = store.status();
        assert_eq!(
            (
                status.skills,
                status.legislation,
                status.natjus,
                status.jurisprudence
            ),
            (1, 1, 1, 1)
        );
        let outcome = store
            .execute("buscar_habilidades", &json!({"consulta":"RMI"}))
            .unwrap();
        assert_eq!(outcome.value["resultados"].as_array().unwrap().len(), 1);
        let read = store
            .execute("ler_habilidade", &json!({"id":"calculo-rmi"}))
            .unwrap();
        assert!(read.value["conteudo"]
            .as_str()
            .unwrap()
            .contains("sintetico"));
        assert_eq!(status.natjus_text, 1);
        let natjus = store.execute("ler_natjus", &json!({"id":"7"})).unwrap();
        assert!(natjus.value["conteudo"]
            .as_str()
            .unwrap()
            .contains("PDF sintetico"));
        let jurisprudence = store
            .execute(
                "buscar_jurisprudencia",
                &json!({"consulta":"coisa julgada"}),
            )
            .unwrap();
        assert_eq!(
            jurisprudence.value["resultados"].as_array().unwrap().len(),
            1
        );
    }

    #[test]
    #[ignore]
    fn real_imported_base_is_readable_and_searchable() {
        let root = std::env::var_os("PREVIDENCIAR_AI_DATA").expect("PREVIDENCIAR_AI_DATA");
        let store = DataStore::new(PathBuf::from(root));
        let status = store.status();
        assert!(status.ready, "{status:?}");
        assert!(status.skills > 1000, "{status:?}");
        assert!(status.legislation > 10, "{status:?}");
        assert!(status.natjus > 10, "{status:?}");
        assert!(status.jurisprudence > 10, "{status:?}");
        let outcome = store
            .execute(
                "buscar_habilidades",
                &json!({"consulta":"aposentadoria por tempo de contribuicao"}),
            )
            .unwrap();
        assert!(!outcome.value["resultados"].as_array().unwrap().is_empty());
        let outcome = store
            .execute(
                "buscar_jurisprudencia",
                &json!({"consulta":"erro medico responsabilidade do Estado"}),
            )
            .unwrap();
        assert!(!outcome.value["resultados"].as_array().unwrap().is_empty());
        let outcome = store
            .execute("buscar_natjus", &json!({"consulta":"vedolizumabe"}))
            .unwrap();
        let results = outcome.value["resultados"].as_array().unwrap();
        assert!(!results.is_empty());
        let id = results[0]["id"].as_str().unwrap();
        let read = store.execute("ler_natjus", &json!({"id": id})).unwrap();
        assert!(read.value["conteudo"].as_str().unwrap().chars().count() > 100);
        let outcome = store
            .execute("buscar_legislacao", &json!({"consulta":"lei"}))
            .unwrap();
        assert!(!outcome.value["resultados"].as_array().unwrap().is_empty());
    }

    #[test]
    fn traversal_and_missing_indexes_are_safe() {
        let (_dir, store) = fixture();
        assert!(store
            .execute("ler_habilidade", &json!({"id":"../secret"}))
            .is_err());
        let empty = DataStore::new(_dir.path().join("missing"));
        assert!(!empty.status().ready);
        assert!(empty
            .execute("buscar_legislacao", &json!({"consulta":"lei"}))
            .is_err());
    }
}


