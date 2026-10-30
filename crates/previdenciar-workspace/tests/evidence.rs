use previdenciar_workspace::evidence::{
    CampoExtraido, ConfiancaCampo, Documento, Fato, Leitura, MotivoPendencia, OrigemCampo,
    OrigemTexto, Pendencia, ProvenienciaFato, StatusLeitura, TipoLeitura,
};
use previdenciar_workspace::{Calculation, Workspace};
use serde_json::json;

fn hash() -> String {
    "a".repeat(64)
}

fn document() -> Documento {
    Documento {
        id: "doc-1".into(),
        nome_arquivo: "autos.pdf".into(),
        mime: "application/pdf".into(),
        paginas: 3,
        origem_texto: OrigemTexto::Ocr,
        ocr_confianca: Some(0.91),
        hash_sha256: hash(),
        criado_em: "2026-10-30T12:00:00Z".into(),
    }
}

fn reading() -> Leitura {
    Leitura {
        id: "read-1".into(),
        documento_id: "doc-1".into(),
        tipo: TipoLeitura::DocumentoGenerico,
        campos: vec![CampoExtraido {
            chave: "fato.data".into(),
            valor: json!("2025-01-10"),
            confianca: ConfiancaCampo::Alta,
            origem: OrigemCampo {
                documento_id: "doc-1".into(),
                pagina: 2,
                trecho: "Data do fato: 10/01/2025".into(),
            },
        }],
        paginas_lidas: 3,
        pendencias: vec![Pendencia {
            campo: "fato.rg".into(),
            motivo: MotivoPendencia::AusenteNoDocumento,
            pergunta: "Qual é o RG?".into(),
        }],
        status: StatusLeitura::EmRevisao,
        texto_extraido: Some("Texto sintético do documento.".into()),
    }
}

fn fact() -> Fato {
    Fato {
        id: "fact-1".into(),
        texto: "O fato ocorreu em 10/01/2025.".into(),
        proveniencia: ProvenienciaFato::Comprovado,
        origem: OrigemCampo {
            documento_id: "doc-1".into(),
            pagina: 2,
            trecho: "Data do fato: 10/01/2025".into(),
        },
        alegado_por: None,
        confirmado: true,
        usado_em: vec![1],
    }
}

fn workspace() -> Workspace {
    let mut workspace = Workspace::default();
    workspace.calculations.push(Calculation {
        id: 1,
        ..Default::default()
    });
    workspace.source_documents.push(document());
    workspace.readings.push(reading());
    workspace.facts.push(fact());
    workspace
}

#[test]
fn valid_evidence_roundtrips_and_is_structurally_accepted() {
    let workspace = workspace();
    let json = serde_json::to_value(&workspace).unwrap();
    let restored: Workspace = serde_json::from_value(json).unwrap();
    assert!(restored.validate().is_empty(), "{:?}", restored.validate());
    assert_eq!(restored.readings[0].campos[0].origem.pagina, 2);
}

#[test]
fn dangling_references_and_bad_provenance_are_rejected() {
    let mut workspace = workspace();
    workspace.source_documents[0].hash_sha256 = "not-a-hash".into();
    workspace.readings[0].documento_id = "missing-doc".into();
    workspace.readings[0].campos[0].origem.pagina = 99;
    workspace.facts[0].proveniencia = ProvenienciaFato::Alegado;
    let messages = workspace.validate();
    assert!(messages.iter().any(|message| message.contains("hash SHA-256")));
    assert!(messages.iter().any(|message| message.contains("documento inexistente")));
    assert!(messages.iter().any(|message| message.contains("Página de origem")));
    assert!(messages.iter().any(|message| message.contains("quem alega")));
}

#[test]
fn duplicate_evidence_ids_and_out_of_range_pages_are_rejected() {
    let mut workspace = workspace();
    let duplicate = workspace.readings[0].clone();
    workspace.readings.push(duplicate);
    workspace.readings[0].paginas_lidas = 4;
    let messages = workspace.validate();
    assert!(messages.iter().any(|message| message.contains("duplicado")));
    assert!(messages.iter().any(|message| message.contains("Páginas lidas")));
}

#[test]
fn calculation_refuses_unconfirmed_reading_and_unconfirmed_inferred_fact() {
    let mut workspace = workspace();
    workspace.calculations[0].reading_ids = vec!["read-1".into()];
    workspace.calculations[0].fact_ids = vec!["fact-1".into()];
    workspace.facts[0].proveniencia = ProvenienciaFato::Inferido;
    workspace.facts[0].confirmado = false;
    let messages = workspace.validate();
    assert!(messages.iter().any(|message| message.contains("leitura ainda em revisão")));
    assert!(messages.iter().any(|message| message.contains("fato inferido sem confirmação")));
    workspace.readings[0].status = StatusLeitura::Confirmada;
    workspace.facts[0].confirmado = true;
    assert!(workspace.validate().is_empty(), "{:?}", workspace.validate());
}
