//! Camada de correção de salários por perfil oficial versionado.
//!
//! O motor histórico corrigia todo o período pelo INPC. A resposta oficial da
//! Fábrica de Cálculos do TRF3 (docs/TRF3_VALIDATION.md) declara séries
//! diferentes por faixa de competência: IPC-R (07/1994–06/1995), INPC
//! (07/1995–03/1996), IGP-DI (04/1996–01/2004), INPC (02/2004–01/2008) e INPC
//! a partir de 02/2008. Este módulo registra, por competência, a série
//! declarada na faixa de vigência do perfil, o período de vigência e a
//! política de escolha de perfil por faixa de DIB.
//!
//! Regras de integridade:
//! - Nenhum coeficiente de série é inventado aqui. INPC (SGS/BCB 188) e IGP-DI
//!   (SGS/BCB 190) vêm do snapshot compilado em [`crate::dados`]. A série IPC-R
//!   não existe no repositório e precisa chegar por arquivo externo versionado
//!   (JSON ou CSV) carregado com [`CatalogoSeries::carregar_json`] /
//!   [`CatalogoSeries::carregar_csv`] e registrado com
//!   [`CatalogoSeries::registrar`].
//! - Sem a série oficial carregada, o fator do perfil **falha de forma
//!   explícita**, nomeando a série ausente e o caminho de carga. Nada é
//!   preenchido com zero nem extrapolado.
//! - O INPC só entra no lugar de série ausente como **fallback documentado**
//!   ([`PoliticaFallback::FallbackInpcDocumentado`]), e cada substituição fica
//!   registrada em [`FatorCorrecao`].
//!
//! Formato JSON aceito por [`CatalogoSeries::carregar_json`]:
//!
//! ```json
//! {
//!   "serie": "IPC-R",
//!   "versao": "2026-10-31",
//!   "fonte": "declaração da origem dos dados",
//!   "valores": [
//!     { "competencia": "1994-07", "variacao": "2.62" }
//!   ]
//! }
//! ```
//!
//! Formato CSV aceito por [`CatalogoSeries::carregar_csv`]: cabeçalho
//! `competencia,variacao` (opcional), uma linha por competência, linhas vazias
//! e comentários iniciados por `#` são ignorados. A série, a versão e a fonte
//! são informadas por parâmetro, pois o CSV não as carrega.
//!
//! A série usada fica registrada em três níveis: [`PassoCorrecao`] (mês a mês
//! do fator), [`crate::rmi::ParcelaPbc`] (por salário do PBC) e
//! [`crate::rmi::PbcResultado`] (consolidação do cálculo).

use crate::calendar::{Competencia, IsoDate};
use crate::money::{add, mul, parse_money, Result, RgpsError};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Índice de correção usado numa faixa de vigência do perfil.
///
/// IPCA está reservado para perfis futuros: não há série IPCA no repositório e
/// nenhum perfil atual usa a faixa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SerieIndice {
    #[serde(rename = "INPC")]
    Inpc,
    #[serde(rename = "IGP-DI")]
    IgpDi,
    #[serde(rename = "IPC-R")]
    IpcR,
    #[serde(rename = "IPCA")]
    Ipca,
}

impl SerieIndice {
    pub fn codigo(self) -> &'static str {
        match self {
            SerieIndice::Inpc => "INPC",
            SerieIndice::IgpDi => "IGP-DI",
            SerieIndice::IpcR => "IPC-R",
            SerieIndice::Ipca => "IPCA",
        }
    }

    /// Aceita variações de escrita: "inpc", "IGPDI", "igp-di", "ipcr", "ipc_r".
    pub fn parse(texto: &str) -> Result<Self> {
        let chave: String = texto
            .trim()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_uppercase();
        match chave.as_str() {
            "INPC" => Ok(SerieIndice::Inpc),
            "IGPDI" => Ok(SerieIndice::IgpDi),
            "IPCR" => Ok(SerieIndice::IpcR),
            "IPCA" => Ok(SerieIndice::Ipca),
            _ => Err(RgpsError(format!(
                "Série de índice desconhecida: {texto}. Séries aceitas: INPC, IGP-DI, IPC-R, IPCA."
            ))),
        }
    }
}

impl std::fmt::Display for SerieIndice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.codigo())
    }
}

/// Origem da tabela de variações mensais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrigemSerie {
    /// Série embutida no binário, gerada a partir do snapshot público declarado em [`crate::dados`].
    Compilada,
    /// Série carregada de arquivo externo versionado (JSON/CSV).
    Externa,
}

impl Default for OrigemSerie {
    fn default() -> Self {
        OrigemSerie::Externa
    }
}

/// Variação mensal de uma série, em percentual (ex.: "2.62" = 2,62%).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValorSerie {
    pub competencia: String,
    pub variacao: String,
}

/// Tabela mensal de uma série de índice, com versão e fonte declaradas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerieMensal {
    pub serie: SerieIndice,
    pub versao: String,
    pub fonte: String,
    #[serde(default)]
    pub origem: OrigemSerie,
    pub valores: Vec<ValorSerie>,
}

impl SerieMensal {
    fn de_tabela(
        serie: SerieIndice,
        versao: &str,
        fonte: &str,
        tabela: &[(&str, &str)],
    ) -> Result<Self> {
        let valores = tabela
            .iter()
            .map(|(competencia, variacao)| ValorSerie {
                competencia: (*competencia).to_string(),
                variacao: (*variacao).to_string(),
            })
            .collect();
        let serie = SerieMensal {
            serie,
            versao: versao.to_string(),
            fonte: fonte.to_string(),
            origem: OrigemSerie::Compilada,
            valores,
        };
        serie.validar()?;
        Ok(serie)
    }

    /// Valida formato, valores e ordem estritamente crescente das competências.
    pub fn validar(&self) -> Result<()> {
        if self.versao.trim().is_empty() {
            return Err(RgpsError(format!(
                "Série {}: versão vazia; toda série carregada precisa ser versionada.",
                self.serie
            )));
        }
        if self.fonte.trim().is_empty() {
            return Err(RgpsError(format!(
                "Série {}: fonte vazia; declare a origem dos dados.",
                self.serie
            )));
        }
        if self.valores.is_empty() {
            return Err(RgpsError(format!(
                "Série {}: sem valores; a série oficial não pode ser carregada vazia.",
                self.serie
            )));
        }
        let mut anterior: Option<String> = None;
        for item in &self.valores {
            Competencia::parse(&item.competencia, &format!("Competência da série {}", self.serie))?;
            parse_money(
                &item.variacao,
                &format!("Variação da série {} em {}", self.serie, item.competencia),
            )?;
            if let Some(ant) = &anterior {
                if item.competencia.as_str() <= ant.as_str() {
                    return Err(RgpsError(format!(
                        "Série {}: competência {} fora de ordem ou duplicada (anterior: {}).",
                        self.serie, item.competencia, ant
                    )));
                }
            }
            anterior = Some(item.competencia.clone());
        }
        Ok(())
    }

    /// Variação mensal da competência, em percentual.
    pub fn variacao(&self, competencia: &str) -> Result<Decimal> {
        match self
            .valores
            .binary_search_by(|item| item.competencia.as_str().cmp(competencia))
        {
            Ok(indice) => parse_money(
                &self.valores[indice].variacao,
                &format!("Variação da série {} em {competencia}", self.serie),
            ),
            Err(_) => Err(RgpsError(format!(
                "Série {} (versão {}) sem variação cadastrada para {competencia}; sem extrapolar nem preencher zero.",
                self.serie, self.versao
            ))),
        }
    }
}

/// Estado de carregamento de uma série no catálogo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EstadoSerie {
    Compilada,
    Externa,
    Ausente,
}

/// Catálogo de séries de índice disponíveis para a correção.
///
/// Começa com INPC e IGP-DI do snapshot compilado. Registro externo sobrepõe a
/// série compilada do mesmo índice (a versão carregada passa a valer).
#[derive(Debug, Clone, Default, Serialize)]
pub struct CatalogoSeries {
    pub series: BTreeMap<SerieIndice, SerieMensal>,
}

impl CatalogoSeries {
    /// Catálogo com as séries embutidas do snapshot (INPC e IGP-DI).
    pub fn compilado() -> Result<Self> {
        let mut series = BTreeMap::new();
        series.insert(
            SerieIndice::Inpc,
            SerieMensal::de_tabela(
                SerieIndice::Inpc,
                "snapshot-2026-10-22",
                crate::dados::inpc::INPC_FONTE,
                &crate::dados::inpc::INPC_MENSAL,
            )?,
        );
        series.insert(
            SerieIndice::IgpDi,
            SerieMensal::de_tabela(
                SerieIndice::IgpDi,
                "snapshot-2026-10-22",
                crate::dados::igpdi::IGPDI_FONTE,
                &crate::dados::igpdi::IGPDI_MENSAL,
            )?,
        );
        Ok(CatalogoSeries { series })
    }

    /// Registra uma série carregada de arquivo externo. Sobrescreve série já
    /// existente do mesmo índice.
    pub fn registrar(&mut self, serie: SerieMensal) -> Result<()> {
        serie.validar()?;
        self.series.insert(serie.serie, serie);
        Ok(())
    }

    pub fn estado(&self, serie: SerieIndice) -> EstadoSerie {
        match self.series.get(&serie) {
            Some(s) => match s.origem {
                OrigemSerie::Compilada => EstadoSerie::Compilada,
                OrigemSerie::Externa => EstadoSerie::Externa,
            },
            None => EstadoSerie::Ausente,
        }
    }

    /// Séries exigidas pelo perfil que não estão carregadas no catálogo.
    pub fn series_ausentes(&self, perfil: &PerfilCorrecao) -> Vec<SerieIndice> {
        perfil
            .series_exigidas()
            .into_iter()
            .filter(|serie| !self.series.contains_key(serie))
            .collect()
    }

    /// Variação mensal da série oficial. Ausência de série ou de mês falha de
    /// forma explícita, nomeando a lacuna e o caminho de carga.
    pub fn variacao(&self, serie: SerieIndice, competencia: &Competencia) -> Result<Decimal> {
        let tabela = self.series.get(&serie).ok_or_else(|| {
            RgpsError(format!(
                "Série oficial {serie} ausente do catálogo de correção. Carregue o arquivo versionado \
                 (JSON/CSV) da série e registre com CatalogoSeries::registrar; sem ela, o fator do perfil \
                 fica bloqueado. O INPC só substitui série ausente por fallback explicitamente documentado."
            ))
        })?;
        tabela.variacao(&competencia.to_string())
    }
}

/// Carrega uma série a partir do JSON versionado (formato documentado no topo do módulo).
pub fn carregar_json(texto: &str) -> Result<SerieMensal> {
    let mut serie: SerieMensal = serde_json::from_str(texto)
        .map_err(|e| RgpsError(format!("JSON de série inválido: {e}")))?;
    serie.origem = OrigemSerie::Externa;
    serie.validar()?;
    Ok(serie)
}

/// Carrega uma série a partir do CSV `competencia,variacao`. Série, versão e
/// fonte vêm dos parâmetros: o CSV não as declara.
pub fn carregar_csv(
    texto: &str,
    serie: SerieIndice,
    versao: &str,
    fonte: &str,
) -> Result<SerieMensal> {
    let mut valores = Vec::new();
    for linha in texto.lines() {
        let limpo = linha.trim();
        if limpo.is_empty() || limpo.starts_with('#') {
            continue;
        }
        let mut partes = limpo.split(',');
        let competencia = partes.next().unwrap_or("").trim();
        let variacao = partes.next().unwrap_or("").trim();
        if partes.next().is_some() {
            return Err(RgpsError(format!(
                "Linha CSV com mais de duas colunas: {limpo}. Use competencia,variacao."
            )));
        }
        if competencia.eq_ignore_ascii_case("competencia") && variacao.eq_ignore_ascii_case("variacao")
        {
            continue;
        }
        valores.push(ValorSerie {
            competencia: competencia.to_string(),
            variacao: variacao.to_string(),
        });
    }
    let serie = SerieMensal {
        serie,
        versao: versao.to_string(),
        fonte: fonte.to_string(),
        origem: OrigemSerie::Externa,
        valores,
    };
    serie.validar()?;
    Ok(serie)
}

/// Carrega uma série de arquivo externo, pelo formato do nome: `.json` ou `.csv`.
pub fn carregar_arquivo(
    caminho: &std::path::Path,
    serie: SerieIndice,
    versao: &str,
    fonte: &str,
) -> Result<SerieMensal> {
    let texto = std::fs::read_to_string(caminho).map_err(|e| {
        RgpsError(format!(
            "Não foi possível ler o arquivo da série {serie} em {}: {e}",
            caminho.display()
        ))
    })?;
    let extensao = caminho
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extensao.as_str() {
        "json" => carregar_json(&texto),
        "csv" => carregar_csv(&texto, serie, versao, fonte),
        outra => Err(RgpsError(format!(
            "Formato de série não suportado: .{outra}. Use .json ou .csv."
        ))),
    }
}

/// Faixa de vigência de uma série dentro do perfil. `fim` ausente = faixa aberta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaixaSerie {
    pub inicio: String,
    #[serde(default)]
    pub fim: Option<String>,
    pub serie: SerieIndice,
}

impl FaixaSerie {
    pub fn contem(&self, competencia: &str) -> bool {
        self.inicio.as_str() <= competencia
            && self
                .fim
                .as_deref()
                .map(|fim| competencia <= fim)
                .unwrap_or(true)
    }
}

/// Perfil oficial versionado: id, versão, fonte da política e faixas de série.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfilCorrecao {
    pub id: String,
    pub versao: String,
    pub descricao: String,
    pub fonte: String,
    pub faixas: Vec<FaixaSerie>,
}

impl PerfilCorrecao {
    pub fn validar(&self) -> Result<()> {
        if self.id.trim().is_empty() || self.versao.trim().is_empty() {
            return Err(RgpsError(
                "Perfil de correção exige id e versão para ser versionado.".into(),
            ));
        }
        if self.faixas.is_empty() {
            return Err(RgpsError(format!(
                "Perfil {} sem faixas de série; nada corrigir.",
                self.id
            )));
        }
        let mut anterior_fim: Option<String> = None;
        let ultima = self.faixas.len() - 1;
        for (indice, faixa) in self.faixas.iter().enumerate() {
            Competencia::parse(&faixa.inicio, &format!("Início da faixa do perfil {}", self.id))?;
            if let Some(fim) = &faixa.fim {
                Competencia::parse(fim, &format!("Fim da faixa do perfil {}", self.id))?;
                if fim.as_str() < faixa.inicio.as_str() {
                    return Err(RgpsError(format!(
                        "Faixa {}/{} do perfil {} termina antes de começar.",
                        faixa.inicio, fim, self.id
                    )));
                }
            }
            if let Some(ant) = &anterior_fim {
                if faixa.inicio.as_str() <= ant.as_str() {
                    return Err(RgpsError(format!(
                        "Faixas do perfil {} sobrepostas ou fora de ordem em {}.",
                        self.id, faixa.inicio
                    )));
                }
            }
            anterior_fim = match &faixa.fim {
                Some(fim) => Some(fim.clone()),
                None => {
                    if indice != ultima {
                        return Err(RgpsError(format!(
                            "Perfil {}: apenas a última faixa pode ser aberta (sem fim).",
                            self.id
                        )));
                    }
                    None
                }
            };
        }
        Ok(())
    }

    /// Série declarada na faixa de vigência que cobre a competência.
    pub fn serie_da_competencia(&self, competencia: &str) -> Result<SerieIndice> {
        Competencia::parse(competencia, "Competência")?;
        for faixa in &self.faixas {
            if faixa.contem(competencia) {
                return Ok(faixa.serie);
            }
        }
        Err(RgpsError(format!(
            "Perfil {} (versão {}) sem faixa de série para a competência {competencia}.",
            self.id, self.versao
        )))
    }

    /// Séries exigidas pelas faixas do perfil, sem repetição e em ordem estável.
    pub fn series_exigidas(&self) -> Vec<SerieIndice> {
        let mut saida: Vec<SerieIndice> = Vec::new();
        for faixa in &self.faixas {
            if !saida.contains(&faixa.serie) {
                saida.push(faixa.serie);
            }
        }
        saida
    }
}

/// Perfil oficial declarado na resposta da Fábrica de Cálculos do TRF3
/// (docs/TRF3_VALIDATION.md): IPC-R (07/1994–06/1995), INPC (07/1995–03/1996),
/// IGP-DI (04/1996–01/2004), INPC (02/2004–01/2008) e INPC a partir de 02/2008.
pub fn perfil_trf3_v1() -> PerfilCorrecao {
    PerfilCorrecao {
        id: "trf3-v1".into(),
        versao: "1".into(),
        descricao: "Séries por faixa de competência declaradas pela API oficial da Fábrica de Cálculos do TRF3.".into(),
        fonte: "docs/TRF3_VALIDATION.md — POST https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-rmi (resposta com política de índices por período).".into(),
        faixas: vec![
            FaixaSerie { inicio: "1994-07".into(), fim: Some("1995-06".into()), serie: SerieIndice::IpcR },
            FaixaSerie { inicio: "1995-07".into(), fim: Some("1996-03".into()), serie: SerieIndice::Inpc },
            FaixaSerie { inicio: "1996-04".into(), fim: Some("2004-01".into()), serie: SerieIndice::IgpDi },
            FaixaSerie { inicio: "2004-02".into(), fim: Some("2008-01".into()), serie: SerieIndice::Inpc },
            FaixaSerie { inicio: "2008-02".into(), fim: None, serie: SerieIndice::Inpc },
        ],
    }
}

/// Resolve um perfil pelo id (tolerante a caixa e espaços). Id desconhecido
/// falha listando os perfis disponíveis.
pub fn perfil_por_id(id: &str) -> Result<PerfilCorrecao> {
    let chave = id.trim().to_ascii_lowercase();
    let perfil = match chave.as_str() {
        "trf3-v1" => perfil_trf3_v1(),
        _ => {
            return Err(RgpsError(format!(
                "Perfil de correção desconhecido: {id}. Perfis disponíveis: trf3-v1."
            )))
        }
    };
    perfil.validar()?;
    Ok(perfil)
}

/// Faixa de vigência da política de escolha de perfil por DIB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaixaDib {
    pub inicio: String,
    #[serde(default)]
    pub fim: Option<String>,
    pub perfil_id: String,
}

/// Política de escolha de perfil de correção por faixa de DIB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliticaEscolhaDib {
    pub faixas: Vec<FaixaDib>,
}

impl PoliticaEscolhaDib {
    pub fn validar(&self) -> Result<()> {
        if self.faixas.is_empty() {
            return Err(RgpsError(
                "Política de escolha de perfil sem faixas de DIB.".into(),
            ));
        }
        let mut anterior: Option<IsoDate> = None;
        let ultima = self.faixas.len() - 1;
        for (indice, faixa) in self.faixas.iter().enumerate() {
            let inicio = IsoDate::parse(&faixa.inicio, "Início da faixa de DIB")?;
            let fim = faixa
                .fim
                .as_deref()
                .map(|f| IsoDate::parse(f, "Fim da faixa de DIB"))
                .transpose()?;
            if let Some(fim) = fim {
                if fim < inicio {
                    return Err(RgpsError(format!(
                        "Faixa de DIB {}/{} termina antes de começar.",
                        faixa.inicio, fim
                    )));
                }
            }
            if let Some(ant) = anterior {
                if inicio <= ant {
                    return Err(RgpsError(
                        "Faixas de DIB da política sobrepostas ou fora de ordem.".into(),
                    ));
                }
            }
            if faixa.perfil_id.trim().is_empty() {
                return Err(RgpsError(
                    "Faixa de DIB sem perfil de correção associado.".into(),
                ));
            }
            anterior = match fim {
                Some(f) => Some(f),
                None => {
                    if indice != ultima {
                        return Err(RgpsError(
                            "Apenas a última faixa de DIB da política pode ser aberta.".into(),
                        ));
                    }
                    None
                }
            };
        }
        Ok(())
    }

    /// Perfil aplicável à DIB. DIB fora das faixas falha de forma explícita.
    pub fn escolher_perfil(&self, dib: &IsoDate) -> Result<String> {
        self.validar()?;
        for faixa in &self.faixas {
            let inicio = IsoDate::parse(&faixa.inicio, "Início da faixa de DIB")?;
            let fim = faixa
                .fim
                .as_deref()
                .map(|f| IsoDate::parse(f, "Fim da faixa de DIB"))
                .transpose()?;
            let coberta = *dib >= inicio && fim.map(|f| *dib <= f).unwrap_or(true);
            if coberta {
                return Ok(faixa.perfil_id.clone());
            }
        }
        Err(RgpsError(format!(
            "DIB {dib} fora das faixas da política de escolha de perfil de correção; nenhuma política é aplicada por inferência."
        )))
    }
}

/// Política vigente: toda DIB a partir de 1994-07-01 usa o perfil `trf3-v1`.
/// DIB anterior ao Plano Real não está coberta por este perfil.
pub fn politica_trf3_v1() -> PoliticaEscolhaDib {
    PoliticaEscolhaDib {
        faixas: vec![FaixaDib {
            inicio: "1994-07-01".into(),
            fim: None,
            perfil_id: "trf3-v1".into(),
        }],
    }
}

/// Política de escolha por faixa de DIB associada a um perfil. Perfil sem
/// política cadastrada falha de forma explícita.
pub fn politica_para_perfil(perfil_id: &str) -> Result<PoliticaEscolhaDib> {
    let chave = perfil_id.trim().to_ascii_lowercase();
    let politica = match chave.as_str() {
        "trf3-v1" => politica_trf3_v1(),
        _ => {
            return Err(RgpsError(format!(
                "Perfil {perfil_id} sem política de escolha por faixa de DIB cadastrada."
            )))
        }
    };
    politica.validar()?;
    Ok(politica)
}

/// Como tratar série oficial ausente durante o cálculo do fator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoliticaFallback {
    /// Padrão: série ausente bloqueia o cálculo, com erro que nomeia a série.
    ExigirSerieOficial,
    /// Fallback documentado: usa o INPC no lugar da série ausente e registra
    /// cada substituição. Não serve para declarar paridade com o cálculo oficial.
    FallbackInpcDocumentado,
}

/// Um mês do fator de correção, com a série declarada no perfil e a série
/// efetivamente usada (diferente apenas em fallback documentado).
#[derive(Debug, Clone, Serialize)]
pub struct PassoCorrecao {
    pub competencia: String,
    pub serie_perfil: String,
    pub serie_usada: String,
    pub variacao: String,
    pub fonte: String,
    pub fallback_inpc: bool,
}

/// Fator de correção de uma competência até a referência, com auditoria das
/// séries usadas e das substituições de fallback.
#[derive(Debug, Clone, Serialize)]
pub struct FatorCorrecao {
    pub perfil_id: String,
    pub perfil_versao: String,
    pub fator: Decimal,
    pub fator_texto: String,
    pub passos: Vec<PassoCorrecao>,
    /// Séries efetivamente usadas, sem repetição, na ordem em que aparecem.
    pub series_usadas: Vec<String>,
    /// Competências corrigidas pelo INPC por falta da série oficial ("AAAA-MM: SÉRIE→INPC").
    pub substituicoes_inpc: Vec<String>,
    /// Séries do perfil que não estavam carregadas no catálogo.
    pub series_ausentes: Vec<String>,
}

/// Fator de correção da `competencia` até a `referencia` (ambas inclusas),
/// aplicando a série da faixa de vigência do perfil para cada mês.
pub fn fator_com_perfil(
    catalogo: &CatalogoSeries,
    perfil: &PerfilCorrecao,
    politica: PoliticaFallback,
    competencia: &Competencia,
    referencia: &Competencia,
) -> Result<FatorCorrecao> {
    perfil.validar()?;
    if competencia > referencia {
        return Err(RgpsError(format!(
            "Competência {competencia} é posterior à referência {referencia}."
        )));
    }
    let mut fator = Decimal::ONE;
    let mut passos: Vec<PassoCorrecao> = Vec::new();
    let mut series_usadas: Vec<String> = Vec::new();
    let mut substituicoes_inpc: Vec<String> = Vec::new();
    let mut series_ausentes: Vec<String> = Vec::new();
    let mut cursor = *competencia;
    while cursor <= *referencia {
        let serie_perfil = perfil.serie_da_competencia(&cursor.to_string())?;
        let (taxa, serie_usada, fonte, fallback) = match catalogo.variacao(serie_perfil, &cursor) {
            Ok(taxa) => {
                let fonte = catalogo
                    .series
                    .get(&serie_perfil)
                    .map(|s| s.fonte.clone())
                    .unwrap_or_default();
                (taxa, serie_perfil, fonte, false)
            }
            Err(erro) => {
                if politica == PoliticaFallback::FallbackInpcDocumentado
                    && serie_perfil != SerieIndice::Inpc
                {
                    let fonte_inpc = catalogo
                        .series
                        .get(&SerieIndice::Inpc)
                        .map(|s| s.fonte.clone())
                        .unwrap_or_default();
                    let taxa = catalogo.variacao(SerieIndice::Inpc, &cursor)?;
                    substituicoes_inpc.push(format!("{}: {}→INPC", cursor, serie_perfil));
                    let codigo = serie_perfil.codigo().to_string();
                    if !series_ausentes.contains(&codigo) {
                        series_ausentes.push(codigo);
                    }
                    (taxa, SerieIndice::Inpc, fonte_inpc, true)
                } else {
                    return Err(erro);
                }
            }
        };
        let codigo_usado = serie_usada.codigo().to_string();
        if !series_usadas.contains(&codigo_usado) {
            series_usadas.push(codigo_usado);
        }
        fator = mul(fator, add(Decimal::ONE, taxa / Decimal::from(100)));
        passos.push(PassoCorrecao {
            competencia: cursor.to_string(),
            serie_perfil: serie_perfil.codigo().to_string(),
            serie_usada: serie_usada.codigo().to_string(),
            variacao: crate::money::fixed(taxa, 2),
            fonte,
            fallback_inpc: fallback,
        });
        cursor = cursor.next();
    }
    Ok(FatorCorrecao {
        perfil_id: perfil.id.clone(),
        perfil_versao: perfil.versao.clone(),
        fator_texto: crate::money::fixed(fator, 6),
        fator,
        passos,
        series_usadas,
        substituicoes_inpc,
        series_ausentes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::money;

    fn comp(v: &str) -> Competencia {
        Competencia::parse(v, "t").unwrap()
    }

    #[test]
    fn serie_indice_parse_e_codigo() {
        assert_eq!(SerieIndice::parse("igp-di").unwrap(), SerieIndice::IgpDi);
        assert_eq!(SerieIndice::parse("IPCR").unwrap(), SerieIndice::IpcR);
        assert_eq!(SerieIndice::parse("inpc").unwrap(), SerieIndice::Inpc);
        assert_eq!(SerieIndice::IpcR.codigo(), "IPC-R");
        assert!(SerieIndice::parse("IGPM").is_err());
    }

    #[test]
    fn faixas_trf3_escolhem_serie_por_competencia_nas_fronteiras() {
        let perfil = perfil_trf3_v1();
        perfil.validar().unwrap();
        let casos = [
            ("1994-07", SerieIndice::IpcR),
            ("1995-06", SerieIndice::IpcR),
            ("1995-07", SerieIndice::Inpc),
            ("1996-03", SerieIndice::Inpc),
            ("1996-04", SerieIndice::IgpDi),
            ("2004-01", SerieIndice::IgpDi),
            ("2004-02", SerieIndice::Inpc),
            ("2008-01", SerieIndice::Inpc),
            ("2008-02", SerieIndice::Inpc),
            ("2025-01", SerieIndice::Inpc),
        ];
        for (mes, esperada) in casos {
            assert_eq!(
                perfil.serie_da_competencia(mes).unwrap(),
                esperada,
                "competência {mes}"
            );
        }
    }

    #[test]
    fn perfil_sem_faixa_para_competencia_fora_do_mapa_falha() {
        let perfil = perfil_trf3_v1();
        let erro = perfil.serie_da_competencia("1994-06").unwrap_err();
        assert!(erro.0.contains("sem faixa de série"), "{erro}");
    }

    #[test]
    fn series_exigidas_do_perfil_trf3() {
        let perfil = perfil_trf3_v1();
        let exigidas = perfil.series_exigidas();
        assert_eq!(
            exigidas,
            vec![SerieIndice::IpcR, SerieIndice::Inpc, SerieIndice::IgpDi]
        );
    }

    #[test]
    fn catalogo_compilado_tem_inpc_e_igpdi_e_nao_tem_ipcr() {
        let catalogo = CatalogoSeries::compilado().unwrap();
        assert_eq!(catalogo.estado(SerieIndice::Inpc), EstadoSerie::Compilada);
        assert_eq!(catalogo.estado(SerieIndice::IgpDi), EstadoSerie::Compilada);
        assert_eq!(catalogo.estado(SerieIndice::IpcR), EstadoSerie::Ausente);
        assert_eq!(catalogo.estado(SerieIndice::Ipca), EstadoSerie::Ausente);
        let perfil = perfil_trf3_v1();
        assert_eq!(
            catalogo.series_ausentes(&perfil),
            vec![SerieIndice::IpcR],
            "a série oficial IPC-R não está no repositório"
        );
    }

    #[test]
    fn serie_oficial_ausente_falha_de_forma_explicita_e_nomeia_a_serie() {
        let catalogo = CatalogoSeries::compilado().unwrap();
        let perfil = perfil_trf3_v1();
        // Janela de 2 meses dentro da faixa IPC-R, sem fallback.
        let erro =
            fator_com_perfil(&catalogo, &perfil, PoliticaFallback::ExigirSerieOficial, &comp("1994-07"), &comp("1994-08"))
                .unwrap_err();
        assert!(erro.0.contains("IPC-R"), "erro deve nomear a série: {erro}");
        assert!(erro.0.contains("Carregue o arquivo"), "{erro}");
    }

    #[test]
    fn fallback_inpc_substitui_a_serie_ausente_e_registra_cada_mes() {
        let catalogo = CatalogoSeries::compilado().unwrap();
        let perfil = perfil_trf3_v1();
        let fator = fator_com_perfil(
            &catalogo,
            &perfil,
            PoliticaFallback::FallbackInpcDocumentado,
            &comp("1995-05"),
            &comp("1995-06"),
        )
        .unwrap();
        assert_eq!(fator.passos.len(), 2);
        for passo in &fator.passos {
            assert_eq!(passo.serie_perfil, "IPC-R");
            assert_eq!(passo.serie_usada, "INPC");
            assert!(passo.fallback_inpc);
        }
        assert_eq!(
            fator.substituicoes_inpc,
            vec!["1995-05: IPC-R→INPC", "1995-06: IPC-R→INPC"]
        );
        assert_eq!(fator.series_ausentes, vec!["IPC-R"]);
        assert_eq!(fator.series_usadas, vec!["INPC"]);
    }

    #[test]
    fn faixa_igpdi_usa_a_serie_real_sem_fallback() {
        let catalogo = CatalogoSeries::compilado().unwrap();
        let perfil = perfil_trf3_v1();
        let fator = fator_com_perfil(
            &catalogo,
            &perfil,
            PoliticaFallback::ExigirSerieOficial,
            &comp("1996-04"),
            &comp("1996-05"),
        )
        .unwrap();
        for passo in &fator.passos {
            assert_eq!(passo.serie_perfil, "IGP-DI");
            assert_eq!(passo.serie_usada, "IGP-DI");
            assert!(!passo.fallback_inpc);
        }
        assert!(fator.substituicoes_inpc.is_empty());
        assert!(fator.series_ausentes.is_empty());
    }

    #[test]
    fn fator_sem_fallback_para_misto_falha_na_serie_ausente() {
        let catalogo = CatalogoSeries::compilado().unwrap();
        let perfil = perfil_trf3_v1();
        // Janela que atravessa IPC-R e IGP-DI: sem a IPC-R, nada é calculado.
        let erro = fator_com_perfil(
            &catalogo,
            &perfil,
            PoliticaFallback::ExigirSerieOficial,
            &comp("1995-06"),
            &comp("1996-04"),
        )
        .unwrap_err();
        assert!(erro.0.contains("IPC-R"), "{erro}");
    }

    #[test]
    fn carregar_json_de_serie_versionada_e_registrar() {
        // Valores sintéticos de teste (2,50% e 1,00%): servem só para validar a
        // carga. Nenhum coeficiente oficial é reproduzido aqui.
        let json = r#"{
            "serie": "IPC-R",
            "versao": "teste-1",
            "fonte": "VALORES SINTÉTICOS DE TESTE",
            "valores": [
                { "competencia": "1994-07", "variacao": "2.50" },
                { "competencia": "1994-08", "variacao": "1.00" }
            ]
        }"#;
        let serie = carregar_json(json).unwrap();
        assert_eq!(serie.serie, SerieIndice::IpcR);
        assert_eq!(serie.versao, "teste-1");
        assert_eq!(serie.origem, OrigemSerie::Externa);
        let mut catalogo = CatalogoSeries::compilado().unwrap();
        catalogo.registrar(serie).unwrap();
        assert_eq!(catalogo.estado(SerieIndice::IpcR), EstadoSerie::Externa);
        let taxa = catalogo
            .variacao(SerieIndice::IpcR, &comp("1994-07"))
            .unwrap();
        assert_eq!(money(taxa), "2.50");
        // Mês fora da carga externa continua falhando de forma explícita.
        assert!(catalogo.variacao(SerieIndice::IpcR, &comp("1994-09")).is_err());
    }

    #[test]
    fn carregar_json_rejeita_serie_desconhecida_vazia_e_fora_de_ordem() {
        assert!(carregar_json(r#"{"serie":"XYZ","versao":"1","fonte":"f","valores":[]}"#).is_err());
        let fora_de_ordem = r#"{
            "serie": "IPC-R", "versao": "1", "fonte": "f",
            "valores": [
                { "competencia": "1994-08", "variacao": "1.00" },
                { "competencia": "1994-07", "variacao": "2.50" }
            ]
        }"#;
        let erro = carregar_json(fora_de_ordem).unwrap_err();
        assert!(erro.0.contains("fora de ordem"), "{erro}");
    }

    #[test]
    fn carregar_csv_valida_colunas_ordem_e_cabecalho() {
        let csv = "competencia,variacao\n# comentário\n1994-07,2.50\n1994-08,1.00\n";
        let serie = carregar_csv(csv, SerieIndice::IpcR, "teste-1", "VALORES SINTÉTICOS DE TESTE").unwrap();
        assert_eq!(serie.valores.len(), 2);
        assert_eq!(serie.valores[0].competencia, "1994-07");
        assert!(carregar_csv("1994-07,1,00\n", SerieIndice::IpcR, "1", "f").is_err());
        assert!(carregar_csv(
            "1994-08,1.00\n1994-08,1.00\n",
            SerieIndice::IpcR,
            "1",
            "f"
        )
        .is_err());
    }

    #[test]
    fn politica_dib_cobre_a_partir_de_1994_07_e_rejeita_antes() {
        let politica = politica_trf3_v1();
        politica.validar().unwrap();
        let dib = IsoDate::parse("2025-01-31", "dib").unwrap();
        assert_eq!(politica.escolher_perfil(&dib).unwrap(), "trf3-v1");
        let limite = IsoDate::parse("1994-07-01", "dib").unwrap();
        assert_eq!(politica.escolher_perfil(&limite).unwrap(), "trf3-v1");
        let antes = IsoDate::parse("1994-06-30", "dib").unwrap();
        let erro = politica.escolher_perfil(&antes).unwrap_err();
        assert!(erro.0.contains("fora das faixas"), "{erro}");
    }

    #[test]
    fn perfil_por_id_desconhecido_lista_os_disponiveis() {
        assert!(perfil_por_id("trf3-v1").is_ok());
        let erro = perfil_por_id("trf3-v2").unwrap_err();
        assert!(erro.0.contains("trf3-v1"), "{erro}");
    }

    #[test]
    fn fator_inclui_a_referencia_e_ordena_passos() {
        let catalogo = CatalogoSeries::compilado().unwrap();
        let perfil = perfil_trf3_v1();
        let fator = fator_com_perfil(
            &catalogo,
            &perfil,
            PoliticaFallback::ExigirSerieOficial,
            &comp("2025-01"),
            &comp("2025-03"),
        )
        .unwrap();
        assert_eq!(fator.passos.len(), 3);
        assert_eq!(fator.passos[0].competencia, "2025-01");
        assert_eq!(fator.passos[2].competencia, "2025-03");
        assert_eq!(fator.passos[0].serie_perfil, "INPC");
    }

    #[test]
    fn competencia_posterior_a_referencia_falha() {
        let catalogo = CatalogoSeries::compilado().unwrap();
        let perfil = perfil_trf3_v1();
        let erro = fator_com_perfil(
            &catalogo,
            &perfil,
            PoliticaFallback::ExigirSerieOficial,
            &comp("2025-03"),
            &comp("2025-01"),
        )
        .unwrap_err();
        assert!(erro.0.contains("posterior à referência"), "{erro}");
    }
}
