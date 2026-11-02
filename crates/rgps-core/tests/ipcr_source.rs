//! Contrato da série oficial IPC-R (agregado 86 do IBGE) contra o loader de
//! perfil de correção.
//!
//! Este teste NÃO prova paridade com o TRF3. Ele verifica apenas o insumo:
//! que o dataset versionado carrega pelo formato real do
//! `rgps_core::perfil_correcao`, cobre os 12 meses da faixa declarada pelo
//! perfil `trf3-v1` (07/1994 a 06/1995), está em ordem, usa percentual e
//! reproduz exatamente a série bruta baixada do IBGE.
//!
//! Proveniência: `data/ipcr-ibge-86-20261102.json` (corpo HTTP exato) e
//! `data/ipcr-ibge-86-20261102.meta.json` (URL, data UTC, bytes, SHA-256).
//! O SHA-256 é recalculado fora do Rust nesta frente (Python, `hashlib`), porque
//! o crate não declara dependência criptográfica e nenhuma foi adicionada.
//! Aqui o teste confere o contrato verificável sem rede e sem dependência nova:
//! o hash registrado tem 64 dígitos hexadecimais, o meta aponta o mesmo corpo
//! bruto e o tamanho gravado bate com o declarado.

use rgps_core::calendar::Competencia;
use rgps_core::money::{money, parse_money};
use rgps_core::perfil_correcao::{
    carregar_json, fator_com_perfil, perfil_trf3_v1, CatalogoSeries, EstadoSerie, PoliticaFallback,
    SerieIndice,
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::path::PathBuf;

const DATASET: &str = "data/ipcr.json";
const BRUTO: &str = "data/ipcr-ibge-86-20261102.json";
const META: &str = "data/ipcr-ibge-86-20261102.meta.json";
const VERSAO: &str = "ibge-86-103-brasil-geral-20261102";
const SHA256_BRUTO: &str = "5CEFB1CABA2B0046A83848161E27A0D8C2F12B3AE3E3BE19B3846B2DBC025B98";

fn caminho(relativo: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relativo)
}

fn ler(relativo: &str) -> String {
    std::fs::read_to_string(caminho(relativo))
        .unwrap_or_else(|e| panic!("não foi possível ler {}: {e}", caminho(relativo).display()))
}

fn comp(v: &str) -> Competencia {
    Competencia::parse(v, "competência").unwrap()
}

#[test]
fn loader_externo_carrega_a_serie_ipcr_do_dataset_versionado() {
    let serie = carregar_json(&ler(DATASET)).expect("dataset IPC-R deve carregar pelo loader externo");
    assert_eq!(serie.serie, SerieIndice::IpcR);
    assert_eq!(serie.versao, VERSAO);
    assert_eq!(serie.valores.len(), 12, "faixa IPC-R do perfil trf3-v1 tem 12 meses");
    assert!(serie.fonte.contains("IBGE"), "fonte deve nomear o provedor: {}", serie.fonte);
    assert!(
        !serie.fonte.contains("TRF3") && !serie.fonte.contains("TRF-3"),
        "números do IBGE não podem ser atribuídos ao TRF3: {}",
        serie.fonte
    );
    // `validar()` já roda dentro de carregar_json e em registrar().
    let mut catalogo = CatalogoSeries::compilado().unwrap();
    catalogo.registrar(serie).unwrap();
    assert_eq!(catalogo.estado(SerieIndice::IpcR), EstadoSerie::Externa);
    assert!(catalogo.series_ausentes(&perfil_trf3_v1()).is_empty());
}

#[test]
fn doze_meses_contiguos_na_ordem_do_perfil() {
    let serie = carregar_json(&ler(DATASET)).unwrap();
    let esperado: Vec<String> = (0..12)
        .map(|i| {
            let mut c = comp("1994-07");
            for _ in 0..i {
                c = c.next();
            }
            c.to_string()
        })
        .collect();
    let obtido: Vec<String> = serie.valores.iter().map(|v| v.competencia.clone()).collect();
    assert_eq!(obtido, esperado, "competências contíguas de 1994-07 a 1995-06");
    assert_eq!(obtido.first().map(String::as_str), Some("1994-07"));
    assert_eq!(obtido.last().map(String::as_str), Some("1995-06"));
    // Estritamente crescente: o loader já rejeita duplicado ou fora de ordem,
    // mas o teste confirma o efeito no dataset publicado.
    for par in obtido.windows(2) {
        assert!(par[0] < par[1], "ordem quebrada entre {} e {}", par[0], par[1]);
    }
}

#[test]
fn unidade_do_dataset_e_percentual_nao_fracao() {
    let serie = carregar_json(&ler(DATASET)).unwrap();
    let julho = serie.variacao("1994-07").unwrap();
    assert_eq!(money(julho), "6.08");
    // Se o valor fosse fração (0,0608), o fator seria 1.000608.
    let fator = Decimal::ONE + julho / Decimal::from(100);
    assert_eq!(
        rgps_core::money::fixed(fator, 4),
        "1.0608",
        "6.08% aplica fator 1.0608"
    );
    // Nenhum mês da faixa pode vir vazio, zero ou negativo.
    for valor in &serie.valores {
        let taxa = parse_money(&valor.variacao, "variação").unwrap();
        assert!(taxa > Decimal::ZERO, "{} sem variação positiva", valor.competencia);
    }
}

#[test]
fn meses_fora_da_cobertura_falham_sem_extrapolar() {
    let serie = carregar_json(&ler(DATASET)).unwrap();
    let antes = serie.variacao("1994-06").unwrap_err();
    let depois = serie.variacao("1995-07").unwrap_err();
    assert!(antes.0.contains("sem variação cadastrada"), "{antes}");
    assert!(depois.0.contains("sem variação cadastrada"), "{depois}");
}

#[test]
fn dataset_reproduz_a_serie_bruta_do_ibge_sem_reordenar_nem_reescrever() {
    let dataset = carregar_json(&ler(DATASET)).unwrap();
    let bruto: Value = serde_json::from_str(&ler(BRUTO)).expect("corpo bruto do IBGE deve ser JSON");
    assert_eq!(bruto[0]["id"], "103");
    assert_eq!(bruto[0]["unidade"], "%");
    assert_eq!(bruto[0]["variavel"], "IPCR - Percentual no mês");
    let classe = &bruto[0]["resultados"][0]["classificacoes"][0];
    assert_eq!(classe["id"], "72");
    assert_eq!(classe["categoria"]["2640"], "Índice geral");
    let localidade = &bruto[0]["resultados"][0]["series"][0]["localidade"];
    assert_eq!(localidade["id"], "1");
    assert_eq!(localidade["nivel"]["id"], "N1");
    let serie_bruta = bruto[0]["resultados"][0]["series"][0]["serie"]
        .as_object()
        .expect("série bruta é objeto AAAAAMM → string");
    assert_eq!(serie_bruta.len(), 12);
    for valor in &dataset.valores {
        let chave = valor.competencia.replace('-', "");
        let original = serie_bruta
            .get(&chave)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("competência {chave} ausente do corpo bruto"));
        assert_eq!(original, valor.variacao, "valor de {chave} foi reescrito");
    }
}

#[test]
fn metadados_declaram_proveniencia_e_o_mesmo_arquivo_bruto() {
    let meta: Value = serde_json::from_str(&ler(META)).expect("meta deve ser JSON");
    assert_eq!(meta["serie"], "IPC-R");
    assert_eq!(
        meta["nome_oficial"], "Índice de Preços ao Consumidor em Real",
        "nome oficial do agregado 86"
    );
    assert_eq!(meta["versao"], VERSAO);
    assert_eq!(meta["metodo"], "GET");
    assert_eq!(meta["http_status"], 200);
    assert_eq!(meta["unidade"], "%");
    assert_eq!(meta["meses"], 12);
    assert_eq!(meta["arquivo_bruto"], "ipcr-ibge-86-20261102.json");
    let url = meta["url"].as_str().unwrap();
    assert!(url.contains("agregados/86"), "{url}");
    assert!(url.contains("variaveis/103"), "{url}");
    assert!(url.contains("classificacao=72"), "{url}");
    assert!(url.contains("N1"), "{url}");
    assert_eq!(meta["sha256_arquivo_bruto"], SHA256_BRUTO);
    assert_eq!(meta["sha256_conteudo"], SHA256_BRUTO);
    assert_eq!(
        SHA256_BRUTO.len(),
        64,
        "SHA-256 tem 64 dígitos hexadecimais"
    );
    assert!(
        SHA256_BRUTO.chars().all(|c| c.is_ascii_hexdigit()),
        "SHA-256 registrado só pode conter hexadecimal"
    );
    assert!(meta["coletado_utc"].as_str().unwrap().ends_with('Z'));
    assert!(meta["versao_declarada_pelo_provedor"].is_null());
    assert!(meta["etag"].is_null());
    assert!(meta["last_modified"].is_null());
    // Tamanho do corpo efetivamente gravado.
    let bruto = std::fs::read(caminho(BRUTO)).unwrap();
    assert_eq!(bruto.len(), meta["bytes_conteudo"].as_u64().unwrap() as usize);
    assert_eq!(bruto.len(), 495);
}

#[test]
fn dataset_nao_carrega_rmi_nem_origem_trf3() {
    let texto = ler(DATASET);
    let minusculo = texto.to_lowercase();
    for proibido in ["rmi", "salario_beneficio", "salário_benefício", "trf3", "trf-3"] {
        assert!(
            !minusculo.contains(proibido),
            "o dataset é só insumo de índice; não pode conter {proibido}"
        );
    }
    let bruto = ler(BRUTO).to_lowercase();
    assert!(!bruto.contains("rmi") && !bruto.contains("trf3"));
}

#[test]
fn com_a_serie_carregada_a_faixa_ipcr_usa_ipcr_sem_fallback() {
    // Unidade do loader com o motor: a faixa 1994-07..1995-06 passa a resolver
    // pela série oficial, sem substituição pelo INPC. Isto NÃO é paridade com o
    // TRF3, que continua pendente de execução multi-caso.
    let mut catalogo = CatalogoSeries::compilado().unwrap();
    catalogo
        .registrar(carregar_json(&ler(DATASET)).unwrap())
        .unwrap();
    let perfil = perfil_trf3_v1();
    let fator = fator_com_perfil(
        &catalogo,
        &perfil,
        PoliticaFallback::ExigirSerieOficial,
        &comp("1994-07"),
        &comp("1995-06"),
    )
    .expect("com a IPC-R carregada, a faixa não deve bloquear");
    assert_eq!(fator.passos.len(), 12);
    for passo in &fator.passos {
        assert_eq!(passo.serie_perfil, "IPC-R");
        assert_eq!(passo.serie_usada, "IPC-R");
        assert!(!passo.fallback_inpc, "{} caiu em fallback", passo.competencia);
        assert!(passo.fonte.contains("IBGE"), "{} sem fonte: {}", passo.competencia, passo.fonte);
    }
    assert!(fator.substituicoes_inpc.is_empty());
    assert!(fator.series_ausentes.is_empty());
    // Fronteira: 1995-07 já é INPC, sem contaminação da IPC-R.
    let julho = fator_com_perfil(
        &catalogo,
        &perfil,
        PoliticaFallback::ExigirSerieOficial,
        &comp("1995-07"),
        &comp("1995-07"),
    )
    .unwrap();
    assert_eq!(julho.passos[0].serie_usada, "INPC");
}
