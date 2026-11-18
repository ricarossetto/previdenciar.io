use crate::atualizacao::{salario_minimo_vigente, teto_vigente, ParcelaCorrigida};
use crate::calendar::{idade_em_meses, Competencia, IsoDate};
use crate::carencia::calcular_carencia;
use crate::especie::{self, Especie};
use crate::money::{add, div, money, mul, parse_money, to_cents, Result, RgpsError};
use crate::rmi::{
    corrigir_parcelas_opcoes, divisor_usado, pbc_de_parcelas_excluindo,
    pbc_de_parcelas_opcoes, pbc_de_parcelas_minimo_opcoes_pub,
    salario_beneficio_sem_correcao, PbcResultado,
};
use crate::tempo::{calcular_tempo, TempoTotal, VinculoEntrada};
use crate::transicoes::{build_transicoes, fator_previdenciario, TransicoesEntrada};
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeSet;

const ANOS_BASE: [(&str, i64); 2] = [("feminino", 15), ("masculino", 20)];
const COMPETENCIA_REFORMA: Competencia = Competencia { ano: 2019, mes: 11 };

#[derive(Debug, Clone)]
pub struct RgpsEntrada {
    pub nome: Option<String>,
    /// Espécie INSS declarada. Só a 42 (aposentadoria por tempo de
    /// contribuição) tem motor aqui: outra espécie bloqueia, nunca é aproximada.
    pub especie: Option<String>,
    pub sexo: String,
    pub data_nascimento: Option<IsoDate>,
    pub data_referencia: Option<IsoDate>,
    pub vinculos: Vec<VinculoEntrada>,
    pub competencias: Vec<(Competencia, Decimal)>,
    pub opcoes: RgpsOpcoes,
}

#[derive(Debug, Clone, Default)]
pub struct RgpsOpcoes {
    pub corrigir_salarios: bool,
    pub descarte: bool,
    pub divisor_minimo: bool,
    /// Reservado para o descarte da EC 103/2019; o motor recalcula tempo,
    /// carência, coeficiente e transições após a exclusão.
    pub descartar_menores_minimo: Option<u32>,
    /// Competências excluídas do PBC e do tempo por valor abaixo do mínimo.
    pub meses_fora_do_tempo: Vec<Competencia>,
    /// Competências com contribuição comprovada; quando presentes, são a base da carência.
    pub competencias_contribuicao: Vec<Competencia>,
    /// Competências excluídas da carência por decisão documentada do caso.
    pub competencias_sem_carencia: Vec<Competencia>,
    pub referencia_correcao: Option<Competencia>,
    pub teto: Option<Decimal>,
    pub piso: Option<Decimal>,
    /// Aplica o teto vigente do salário de contribuição na competência da DER.
    pub aplicar_teto_vigente_der: bool,
    pub expectativa_sobrevida: Option<Decimal>,
    pub periodos_graca: Vec<(IsoDate, IsoDate)>,
    pub recolhimentos_atraso: Vec<Competencia>,
    pub regra_alvo: Option<String>,
    /// Origem declarada da incapacidade (31/32): comum, acidente,
    /// doenca_ocupacional, doenca_do_trabalho ou doenca_listada.
    pub incapacidade_origem: Option<String>,
    /// 46: grau/agente declarado — "15", "20" ou "25" anos.
    pub especial_grau: Option<String>,
    /// 46: tempo de efetiva exposição declarado, em meses.
    pub tempo_especial_meses: Option<i64>,
    /// 46: tempo especial completo até 13/11/2019.
    pub especial_antes_reforma: Option<bool>,
    /// 57: tempo de efetivo magistério declarado, em meses.
    pub magisterio_meses: Option<i64>,
    /// 57: magistério exclusivo.
    pub magisterio_exclusivo: Option<bool>,
    /// 57: tempo de magistério completo até 13/11/2019.
    pub magisterio_antes_reforma: Option<bool>,
    /// Perfil de correção versionado (ex.: "trf3-v1"). Quando presente com
    /// `corrigir_salarios`, a correção usa as séries por faixa de vigência do
    /// perfil em vez do INPC único. Ausente = caminho histórico de INPC único.
    pub perfil_correcao: Option<String>,
    /// Fallback documentado: substitui série oficial ausente pelo INPC e
    /// registra cada substituição. Sem isso, a série ausente bloqueia o cálculo.
    pub fallback_inpc_documentado: bool,
    /// Descarte ótimo automático da EC 103/2019, art. 26, § 6º (maximiza a RMI mantendo tempo e carência).
    pub descarte_otimo: bool,
    /// Aplica automaticamente o salário mínimo vigente na competência da DIB/referência quando `piso` for None.
    pub aplicar_piso_automatico: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulacaoDescarteView {
    pub n: usize,
    pub competencia_removida: Option<String>,
    pub valor_removido: Option<String>,
    pub tempo_meses: i64,
    pub carencia_meses: usize,
    pub divisor: u32,
    pub salario_beneficio: String,
    pub regra_escolhida: Option<String>,
    pub rmi: Option<String>,
    pub aceita: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoriaItem {
    pub tipo: &'static str,
    pub descricao: String,
    pub valor: String,
}

#[derive(Debug, Serialize)]
pub struct RgpsResultado {
    pub segurado: SeguradoView,
    pub especie: EspecieView,
    pub primeira_competencia: String,
    pub ultima_competencia: String,
    pub parametros: ParametrosView,
    pub resumo: ResumoView,
    pub tempo: Option<serde_json::Value>,
    pub carencia: serde_json::Value,
    pub correcao: Option<serde_json::Value>,
    pub memoria: Vec<MemoriaItem>,
    pub transicoes: Option<serde_json::Value>,
    /// Requisitos e regras próprias das espécies 41, 32 e 31.
    pub beneficio: Option<serde_json::Value>,
    /// Etapas registradas na simulação de descarte da EC 103/2019.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub simulacao_descartes: Vec<SimulacaoDescarteView>,
    pub avisos: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EspecieView {
    pub codigo: String,
    pub nome: String,
    pub regra_escolhida: Option<String>,
    /// Nome legível da regra escolhida (41/32/31); a 42 usa o comparador de transições.
    pub regra_nome: Option<String>,
    pub elegivel: bool,
}

#[derive(Debug, Serialize)]
pub struct SeguradoView {
    pub nome: Option<String>,
    pub sexo: String,
    pub data_nascimento: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParametrosView {
    pub divisor: u32,
    pub divisor_minimo_aplicado: bool,
    pub anos: i64,
    pub meses_restantes: i64,
    pub anos_base: i64,
    pub coeficiente: String,
    /// Coeficiente efetivamente aplicado pela regra vencedora, quando diferente do coeficiente da espécie.
    pub coeficiente_regra: Option<String>,
    pub correcao_ativa: bool,
    pub referencia_correcao: Option<String>,
    pub descarte_aplicado: Option<bool>,
    /// Perfil de correção versionado usado na correção dos salários.
    pub perfil_correcao: Option<String>,
    /// Fallback documentado pelo INPC habilitado junto ao perfil.
    pub fallback_inpc_documentado: bool,
}

#[derive(Debug, Serialize)]
pub struct ResumoView {
    pub competencias: usize,
    pub carencia_meses: usize,
    pub soma_salarios: String,
    pub salario_beneficio: String,
    pub coeficiente: String,
    pub rmi_bruta: String,
    pub rmi: String,
    pub limite_aplicado: Option<&'static str>,
    pub teto: Option<String>,
    pub piso: Option<String>,
    /// Regra de transição vencedora, quando houver: a RMI exibida é a dela.
    pub regra_escolhida: Option<String>,
    pub regra_nome: Option<String>,
}

fn anos_base(sexo: &str) -> i64 {
    ANOS_BASE
        .iter()
        .find(|(s, _)| *s == sexo)
        .map(|(_, v)| *v)
        .unwrap_or(20)
}

/// Média dos últimos 12 salários de contribuição, corrigidos quando o PBC foi
/// corrigido. Usada no limite do auxílio por incapacidade temporária
/// (Lei 8.213/1991, art. 29, § 10); não substitui o salário de benefício.
fn media_ultimos_12(
    competencias: &[(Competencia, Decimal)],
    correcao: Option<&crate::rmi::PbcResultado>,
) -> Option<Decimal> {
    let valores: Vec<Decimal> = match correcao {
        Some(pbc) => {
            let mut parcelas: Vec<&crate::rmi::ParcelaPbc> = pbc.parcelas.iter().collect();
            parcelas.sort_by(|a, b| a.competencia.cmp(&b.competencia));
            parcelas
                .iter()
                .rev()
                .take(12)
                .filter_map(|p| parse_money(&p.corrigido, "média dos últimos 12").ok())
                .collect()
        }
        None => competencias
            .iter()
            .rev()
            .take(12)
            .map(|(_, valor)| *valor)
            .collect(),
    };
    if valores.is_empty() {
        return None;
    }
    let total = valores.iter().copied().fold(Decimal::ZERO, add);
    div(total, Decimal::from(valores.len() as u64)).ok()
}

/// Valida a entrada (porta de validate() do previdenciario.mjs).
pub fn validar(entrada: &RgpsEntrada) -> Result<()> {
    if !crate::SEXOS.contains(&entrada.sexo.as_str()) {
        return Err(RgpsError(format!("Sexo inválido: {}.", entrada.sexo)));
    }
    if entrada.competencias.is_empty() {
        return Err(RgpsError(
            "Informe ao menos uma competência com salário de contribuição.".into(),
        ));
    }
    if let Some(especie) = entrada.especie.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Especie::parse(Some(especie))?;
    }
    let mut vistas: std::collections::BTreeSet<Competencia> = std::collections::BTreeSet::new();
    let primeira = Competencia::parse(crate::PRIMEIRA_COMPETENCIA, "primeira competência")?;
    for (competencia, salario) in &entrada.competencias {
        if *competencia < primeira {
            return Err(RgpsError(format!(
                "Competência {competencia} é anterior a {}; períodos anteriores não entram no PBC.",
                crate::PRIMEIRA_COMPETENCIA
            )));
        }
        if !vistas.insert(*competencia) {
            return Err(RgpsError(format!(
                "Competência duplicada: {competencia}. Concomitância exige conferência manual."
            )));
        }
        if *salario < Decimal::ZERO {
            return Err(RgpsError(format!(
                "Salário da competência {competencia} não pode ser negativo."
            )));
        }
        if salario.is_zero() {
            return Err(RgpsError(format!(
                "Salário da competência {competencia} deve ser maior que zero."
            )));
        }
    }
    if let (Some(teto), Some(piso)) = (entrada.opcoes.teto, entrada.opcoes.piso) {
        if piso > teto {
            return Err(RgpsError("Piso informado é maior que o teto.".into()));
        }
    }
    // Perfil de correção é validado aqui, no núcleo, e não apenas no adaptador:
    // a chamada direta a `calcular` não pode ignorar opções incoerentes em
    // silêncio nem descobrir um perfil desconhecido no meio da apuração. Fica no
    // fim para não alterar a precedência das mensagens de validação já testadas.
    let perfil_informado = entrada
        .opcoes
        .perfil_correcao
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    match perfil_informado {
        Some(perfil) => {
            if !entrada.opcoes.corrigir_salarios {
                return Err(RgpsError(format!(
                    "Perfil de correção {perfil} informado com corrigir_salarios=false: o perfil só se aplica com a correção ativa. Ligue corrigir_salarios ou remova o perfil; a opção não é ignorada em silêncio."
                )));
            }
            // Id desconhecido falha listando os disponíveis, antes de qualquer cálculo.
            crate::perfil_correcao::perfil_por_id(perfil)?;
        }
        None => {
            if entrada.opcoes.fallback_inpc_documentado {
                return Err(RgpsError(
                    "Fallback documentado pelo INPC exige perfil de correção: informe opcoes.perfil_correcao junto com corrigir_salarios, ou desligue o fallback.".into(),
                ));
            }
        }
    }
    Ok(())
}

/// Cálculo RGPS completo: valida, apura tempo, carência, PBC, coeficiente,
/// RMI com limites, transições EC 103 com RMI por regra e vencedora.
pub fn calcular(entrada: &RgpsEntrada) -> Result<RgpsResultado> {
    validar(entrada)?;
    let especie = Especie::parse(entrada.especie.as_deref())?;

    let mut competencias = entrada.competencias.clone();
    competencias.sort_by_key(|(c, _)| *c);
    let mut quantidade = competencias.len();
    let primeira = competencias.first().unwrap().0;
    let ultima = competencias.last().unwrap().0;

    // Tempo de contribuição (opcional).
    let mut tempo = if entrada.vinculos.is_empty() {
        None
    } else {
        Some(calcular_tempo(&entrada.vinculos, entrada.data_referencia.as_ref())?)
    };
    // Competências abaixo do mínimo não contam tempo nem carência: o PBC já as
    // removeu no adaptador; aqui o tempo é ajustado pelo mesmo conjunto.
    if !entrada.opcoes.meses_fora_do_tempo.is_empty() {
        if let Some(t) = tempo.as_mut() {
            let count = entrada.opcoes.meses_fora_do_tempo.len();
            for competencia in &entrada.opcoes.meses_fora_do_tempo {
                t.total_dias_efetivos = (t.total_dias_efetivos - 30).max(0);
                t.total_360 = (t.total_360 - 30).max(0);
                if *competencia <= COMPETENCIA_REFORMA {
                    t.marcos.ate_ec103 = (t.marcos.ate_ec103 - 30).max(0);
                } else {
                    t.marcos.apos_ec103 = (t.marcos.apos_ec103 - 30).max(0);
                }
            }
            let anos = t.total_360 / 360;
            let meses = (t.total_360 % 360) / 30;
            let dias = t.total_360 - anos * 360 - meses * 30;
            t.total = TempoTotal { anos, meses, dias };
            t.carencia_meses = t.carencia_meses.saturating_sub(count);
            t.avisos.push(format!(
                "{count} competência(s) abaixo do mínimo ficaram fora do PBC e do tempo; os vínculos de origem permanecem listados."
            ));
        }
    }
    let inicio_filiacao = entrada
        .vinculos
        .iter()
        .map(|v| v.inicio)
        .min()
        .or_else(|| {
            competencias.first().map(|(c, _)| IsoDate {
                ano: c.ano as i32,
                mes: c.mes,
                dia: 1,
            })
        });
    // Só trata como rural quando TODOS os vínculos são rural/especial; mistura
    // exige análise própria e não é presumida.
    let rural = !entrada.vinculos.is_empty()
        && entrada
            .vinculos
            .iter()
            .all(|v| matches!(v.tipo.as_str(), "rural" | "especial"));

    // Carência qualificada: salários ∪ vínculos ∪ graça ∪ atraso.
    let vinculos_ranges: Vec<(IsoDate, IsoDate)> = entrada
        .vinculos
        .iter()
        .map(|v| {
            let fim = v
                .fim
                .or(entrada.data_referencia)
                .ok_or_else(|| RgpsError("Vínculo sem fim exige data de referência.".into()))?;
            Ok((v.inicio, fim))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut carencia = calcular_carencia(
        &competencias.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        &vinculos_ranges,
        &entrada.opcoes.periodos_graca,
        &entrada.opcoes.recolhimentos_atraso,
    )?;
    // Carência é contagem de competências de contribuição, não união de todos os
    // meses de vínculo. Os meses abaixo do mínimo continuam sendo contribuições
    // para a carência, ainda que fiquem fora do PBC/tempo.
    // Carência por conjunto único (sem dupla contagem quando `meses_fora_do_tempo`
    // ou `competencias_sem_carencia` se sobrepõem às competências listadas).
    let mut competencias_carencia: BTreeSet<Competencia> =
        competencias.iter().map(|(c, _)| *c).collect();
    competencias_carencia.extend(entrada.opcoes.meses_fora_do_tempo.iter().copied());
    let listadas: BTreeSet<Competencia> = entrada
        .opcoes
        .competencias_contribuicao
        .iter()
        .copied()
        .collect();
    let mut base_carencia = if listadas.len() >= competencias_carencia.len() {
        listadas
    } else {
        competencias_carencia
    };
    for sem in &entrada.opcoes.competencias_sem_carencia {
        base_carencia.remove(sem);
    }
    let contribuicoes = base_carencia.len();
    carencia.competencias_salario = contribuicoes;
    carencia.competencias_vinculo = 0;
    let mut carencia_meses = contribuicoes
        .saturating_add(carencia.competencias_graca)
        .saturating_add(carencia.recolhimentos_atraso);

    let reforma_ec103 = IsoDate::parse(crate::REFORMA_EC103, "reforma")?;
    let pre_reforma_dib = especie == Especie::TempoContribuicao
        && entrada.data_referencia.is_some_and(|d| d < reforma_ec103);
    let referencia_dib = entrada.data_referencia.map(|d| Competencia {
        ano: d.ano as u16,
        mes: d.mes,
    });

    // Limites vigentes derivados antes do PBC (para o descarte ótimo avaliar RMI final).
    let referencia_limites = entrada
        .opcoes
        .referencia_correcao
        .or(referencia_dib)
        .unwrap_or(ultima);
    let teto_derivado = if entrada.opcoes.aplicar_teto_vigente_der {
        Some(teto_vigente(&referencia_limites)?)
    } else {
        entrada.opcoes.teto
    };
    let piso_derivado = match entrada.opcoes.piso {
        Some(p) => Some(p),
        None if entrada.opcoes.aplicar_piso_automatico => {
            Some(salario_minimo_vigente(&referencia_limites)?)
        }
        None => None,
    };

    // Perfil de correção (opcional) e catálogo para PBC e direito adquirido.
    let perfil_carregado = match entrada
        .opcoes
        .perfil_correcao
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(perfil_id) => {
            let dib = entrada.data_referencia.ok_or_else(|| {
                RgpsError(
                    "Perfil de correção exige data de referência (DIB/DER) para a política de escolha por faixa de DIB.".into(),
                )
            })?;
            let perfil = crate::perfil_correcao::perfil_por_id(perfil_id)?;
            let politica_dib = crate::perfil_correcao::politica_para_perfil(&perfil.id)?;
            let perfil_da_dib = politica_dib.escolher_perfil(&dib)?;
            if perfil_da_dib != perfil.id {
                return Err(RgpsError(format!(
                    "DIB {dib} escolhe o perfil {perfil_da_dib} na política de faixas, não {perfil_id}."
                )));
            }
            let mut catalogo = crate::perfil_correcao::CatalogoSeries::compilado()?;
            let serie_ipcr = crate::perfil_correcao::carregar_json(include_str!("../data/ipcr.json"))?;
            catalogo.registrar(serie_ipcr)?;
            let politica_fallback = if entrada.opcoes.fallback_inpc_documentado {
                crate::perfil_correcao::PoliticaFallback::FallbackInpcDocumentado
            } else {
                crate::perfil_correcao::PoliticaFallback::ExigirSerieOficial
            };
            Some((catalogo, perfil, politica_fallback))
        }
        None => None,
    };

    // Direito adquirido pré-EC 103 (até 13/11/2019) com reajuste oficial até a DIB.
    let (rmi_direito_adquirido, obs_direito_adquirido) = calcular_direito_adquirido(
        entrada,
        especie,
        &competencias,
        tempo.as_ref(),
        inicio_filiacao,
        perfil_carregado.as_ref().map(|(c, p, f)| (c, p, *f)),
    );

    // PBC / salário de benefício + descarte ótimo automático da EC 103, art. 26, § 6º.
    let correcao_ativa = entrada.opcoes.corrigir_salarios;
    let mut simulacao_descartes: Vec<SimulacaoDescarteView> = Vec::new();
    let correcao: Option<PbcResultado> = if correcao_ativa {
        let referencia_correcao = entrada
            .opcoes
            .referencia_correcao
            .or_else(|| referencia_dib.filter(|d| *d >= ultima))
            .unwrap_or(ultima);
        let perfil_ref = perfil_carregado
            .as_ref()
            .map(|(cat, perf, pol)| (cat, perf, *pol));
        let (parcelas, ref_efetiva, subst_inpc, series_aus) =
            corrigir_parcelas_opcoes(&competencias, &referencia_correcao, true, perfil_ref)?;

        let mut pbc_res = if let Some(minimo) = entrada.opcoes.descartar_menores_minimo {
            pbc_de_parcelas_minimo_opcoes_pub(
                parcelas,
                &referencia_correcao,
                minimo,
                entrada.opcoes.divisor_minimo,
                ref_efetiva.clone(),
            )?
        } else if entrada.opcoes.descarte_otimo
            && especie != Especie::IncapacidadeTemporaria
            && !pre_reforma_dib
        {
            simular_descarte_otimo(
                entrada,
                especie,
                &competencias,
                &parcelas,
                &referencia_correcao,
                tempo.as_ref(),
                carencia_meses,
                inicio_filiacao,
                rural,
                teto_derivado,
                piso_derivado,
                ref_efetiva.clone(),
                rmi_direito_adquirido,
                obs_direito_adquirido.as_deref(),
                &mut simulacao_descartes,
            )?
        } else {
            let descarte_pbc =
                entrada.opcoes.descarte || (pre_reforma_dib && entrada.opcoes.descarte_otimo);
            pbc_de_parcelas_opcoes(
                parcelas,
                &referencia_correcao,
                descarte_pbc,
                entrada.opcoes.divisor_minimo,
                ref_efetiva.clone(),
            )?
        };
        if !subst_inpc.is_empty() {
            pbc_res.substituicoes_inpc = subst_inpc;
        }
        if !series_aus.is_empty() {
            pbc_res.series_ausentes = series_aus;
        }
        Some(pbc_res)
    } else {
        None
    };

    let salario_beneficio = match &correcao {
        Some(pbc) => parse_money(&pbc.salario_beneficio, "salário de benefício")?,
        None => salario_beneficio_sem_correcao(&competencias, entrada.opcoes.divisor_minimo)?,
    };

    // EC 103, art. 26, §6º: o descarte dos menores remove as competências do
    // PBC, do tempo e ajusta a contagem considerada após a exclusão.
    if let Some(pbc) = correcao.as_ref() {
        let aplicar_reducao_ec103 = !pre_reforma_dib
            && (entrada.opcoes.descartar_menores_minimo.is_some() || entrada.opcoes.descarte_otimo);
        if aplicar_reducao_ec103 && !pbc.parcelas_descartadas.is_empty() {
            let discarded: BTreeSet<Competencia> = pbc
                .parcelas_descartadas
                .iter()
                .filter_map(|value| Competencia::parse(value, "competência descartada").ok())
                .collect();
            if !discarded.is_empty() {
                let count = discarded.len();
                competencias.retain(|(competencia, _)| !discarded.contains(competencia));
                quantidade = competencias.len();
                if let Some(t) = tempo.as_mut() {
                    for competencia in &discarded {
                        t.total_dias_efetivos = (t.total_dias_efetivos - 30).max(0);
                        t.total_360 = (t.total_360 - 30).max(0);
                        if *competencia <= COMPETENCIA_REFORMA {
                            t.marcos.ate_ec103 = (t.marcos.ate_ec103 - 30).max(0);
                        } else {
                            t.marcos.apos_ec103 = (t.marcos.apos_ec103 - 30).max(0);
                        }
                    }
                    let anos = t.total_360 / 360;
                    let meses = (t.total_360 % 360) / 30;
                    let dias = t.total_360 - anos * 360 - meses * 30;
                    t.total = TempoTotal { anos, meses, dias };
                    t.carencia_meses = t.carencia_meses.saturating_sub(count);
                    t.avisos.push(format!(
                        "{count} competência(s) descartada(s) do PBC; o tempo foi reduzido em {count} mês(es) e o coeficiente/transições foram recalculados. Os vínculos de origem permanecem listados."
                    ));
                }
                carencia_meses = carencia_meses.saturating_sub(count);
            }
        }
    }
    carencia.carencia_meses = carencia_meses;
    if let Some(t) = tempo.as_mut() {
        t.carencia_meses = carencia_meses;
    }
    let tempo_meses = tempo
        .as_ref()
        .map(|t| (t.marcos.ate_ec103 + t.marcos.apos_ec103) / 30)
        .unwrap_or(0);

    // Coeficiente: a espécie 42 mantém a paridade por competências; 41, 32 e 31
    // usam a regra própria (idade/incapacidade/91%), com requisitos explícitos.
    let media_12 = media_ultimos_12(&competencias, correcao.as_ref());
    let avaliacao = especie::avaliar(&especie::AvaliacaoEntrada {
        especie,
        sexo: &entrada.sexo,
        data_nascimento: entrada.data_nascimento,
        data_referencia: entrada.data_referencia,
        carencia_meses: carencia_meses as i64,
        tempo_meses,
        competencias: quantidade as i64,
        inicio_filiacao,
        rural,
        incapacidade_origem: entrada.opcoes.incapacidade_origem.as_deref(),
        media_ultimos_12: media_12,
        especial_grau: entrada.opcoes.especial_grau.as_deref(),
        especial_meses: entrada.opcoes.tempo_especial_meses,
        especial_antes_reforma: entrada.opcoes.especial_antes_reforma,
        magisterio_meses: entrada.opcoes.magisterio_meses,
        magisterio_exclusivo: entrada.opcoes.magisterio_exclusivo,
        magisterio_antes_reforma: entrada.opcoes.magisterio_antes_reforma,
        tempo_ate_reforma_meses: tempo
            .as_ref()
            .map(|t| t.marcos.ate_ec103 / 30)
            .unwrap_or(0),
    })?;
    let tempo_meses_para_coeficiente = tempo
        .as_ref()
        .map(|t| (t.total.anos as i64) * 12 + t.total.meses as i64)
        .unwrap_or(0);
    let (anos, meses_restantes) = if tempo_meses_para_coeficiente > 0 {
        ((tempo_meses_para_coeficiente / 12) as i64, (tempo_meses_para_coeficiente % 12) as i64)
    } else {
        ((quantidade / 12) as i64, (quantidade % 12) as i64)
    };
    let base = if pre_reforma_dib {
        if entrada.sexo == "feminino" { 30 } else { 35 }
    } else {
        anos_base(&entrada.sexo)
    };
    let excedente = (anos - base).max(0);
    let coeficiente = if especie == Especie::TempoContribuicao {
        if pre_reforma_dib {
            Decimal::from(100)
        } else {
            Decimal::from(60) + Decimal::from(2) * Decimal::from(excedente)
        }
    } else {
        avaliacao.coeficiente
    };

    let mut rmi_bruta = mul(
        salario_beneficio,
        div(coeficiente, Decimal::from(100))?,
    );
    let mut avisos = Vec::new();
    let mut regra_exibicao: Option<String> = None;
    let mut regra_nome_exibicao: Option<String> = None;
    let mut fator_exibicao: Option<Decimal> = None;
    let mut pre_reforma_elegivel = false;

    if pre_reforma_dib {
        let data_ref = entrada
            .data_referencia
            .expect("pre_reforma_dib exige data_referencia");
        let tempo_min_meses = if entrada.sexo == "feminino" { 360 } else { 420 };
        let tempo_meses_pre = tempo
            .as_ref()
            .map(|t| t.total_360 / 30)
            .unwrap_or(quantidade as i64);
        pre_reforma_elegivel = tempo_meses_pre >= tempo_min_meses && carencia_meses >= 180;
        if let Some(nascimento) = entrada.data_nascimento {
            let idade_meses = idade_em_meses(&nascimento, &data_ref);
            let ref_str = data_ref.to_string();
            let es_opt = entrada.opcoes.expectativa_sobrevida.or_else(|| {
                let idade_anos = (idade_meses / 12).clamp(0, 90) as usize;
                crate::dados::expectativa_sobrevida::EXPECTATIVA_GERAL
                    .iter()
                    .filter(|(v, _)| *v <= ref_str.as_str())
                    .map(|(_, row)| row)
                    .last()
                    .and_then(|row| {
                        let val = row[idade_anos];
                        if val.is_empty() {
                            None
                        } else {
                            Decimal::from_str_exact(val).ok()
                        }
                    })
            });
            if let Some(es) = es_opt {
                let tempo_dec_pre = tempo
                    .as_ref()
                    .map(|t| Decimal::from(t.total_360) / Decimal::from(30))
                    .unwrap_or_else(|| Decimal::from(quantidade as i64));
                let fator_bruto =
                    fator_previdenciario(&entrada.sexo, tempo_dec_pre, idade_meses, es)?;
                let fator_trunc = fator_bruto.trunc_with_scale(4);
                let mp676_dt = IsoDate::parse("2015-06-18", "Lei 13.183/2015")?;
                let marco_86_96_dt = IsoDate::parse("2018-12-31", "Lei 13.183/2015 86/96")?;
                let meta_pontos = if data_ref >= marco_86_96_dt {
                    Some(if entrada.sexo == "feminino" { 86i64 } else { 96i64 })
                } else if data_ref >= mp676_dt {
                    Some(if entrada.sexo == "feminino" { 85i64 } else { 95i64 })
                } else {
                    None
                };
                let pontos_meses = tempo_meses_pre + idade_meses;
                let afastado_pontos = meta_pontos.is_some_and(|meta| {
                    tempo_meses_pre >= tempo_min_meses
                        && pontos_meses >= meta * 12
                        && fator_trunc < Decimal::ONE
                });
                let fator_efetivo = if afastado_pontos {
                    Decimal::ONE
                } else {
                    fator_trunc
                };
                rmi_bruta = to_cents(mul(salario_beneficio, fator_efetivo));
                fator_exibicao = Some(fator_efetivo);
                if pre_reforma_elegivel {
                    if afastado_pontos {
                        let meta = meta_pontos.unwrap();
                        regra_exibicao = Some("pre-ec103-pontos".into());
                        regra_nome_exibicao = Some(format!(
                            "Aposentadoria por tempo de contribuição pré-EC 103 ({meta} pontos — Lei 13.183/2015, fator afastado de {:.4})",
                            fator_trunc
                        ));
                    } else {
                        regra_exibicao = Some("pre-ec103-fator".into());
                        regra_nome_exibicao = Some(format!(
                            "Aposentadoria por tempo de contribuição pré-EC 103 (Lei 8.213/1991, fator {:.4})",
                            fator_efetivo
                        ));
                    }
                }
            } else {
                avisos.push(
                    "Tábua IBGE não cobre a idade/data desta DIB para o fator previdenciário pré-EC 103."
                        .into(),
                );
                if pre_reforma_elegivel {
                    regra_exibicao = Some("pre-ec103-fator".into());
                    regra_nome_exibicao = Some(
                        "Aposentadoria por tempo de contribuição pré-EC 103 (sem tábua IBGE para fator)"
                            .into(),
                    );
                }
            }
        } else {
            avisos.push(
                "Sem data de nascimento informada, o fator previdenciário pré-EC 103 não pode ser apurado."
                    .into(),
            );
            if pre_reforma_elegivel {
                regra_exibicao = Some("pre-ec103-fator".into());
                regra_nome_exibicao = Some("Aposentadoria por tempo de contribuição pré-EC 103".into());
            }
        }
        if !pre_reforma_elegivel {
            avisos.push(
                "Requisitos de tempo de contribuição/carência pré-EC 103 não cumpridos na DIB; o valor calculado é uma simulação condicional."
                    .into(),
            );
        }
    }

    let mut limite_aplicado: Option<&'static str> = None;

    // Espécie 31: a RMI não pode exceder a média dos últimos 12 salários
    // (Lei 8.213/1991, art. 29, § 10).
    if avaliacao.limite_media_12 {
        if let Some(media) = media_12 {
            if rmi_bruta > media {
                rmi_bruta = media;
                limite_aplicado = Some("media_12");
            }
        }
    }

    if let Some(teto) = teto_derivado {
        if rmi_bruta > teto {
            rmi_bruta = teto;
            limite_aplicado = Some("teto");
        }
    }
    if let Some(piso) = piso_derivado {
        if limite_aplicado.is_none() && rmi_bruta < piso {
            rmi_bruta = piso;
            limite_aplicado = Some("piso");
        }
    }

    // A RMI exibida é a da regra de transição vencedora; sem regra, fica a RMI
    // da espécie. A memória preserva a RMI bruta/base para auditoria.
    let mut rmi_exibicao = rmi_bruta;
    let mut coeficiente_exibicao = coeficiente;

    // Transições EC 103 (quando há nascimento e DIB a partir de 13/11/2019).
    let mut transicoes = None;
    if let Some(nascimento) = entrada
        .data_nascimento
        .filter(|_| especie == Especie::TempoContribuicao && !pre_reforma_dib)
    {
        let data_referencia = entrada.data_referencia.ok_or_else(|| {
            RgpsError(
                "Informe a data de referência (DER) para comparar as regras de transição.".into(),
            )
        })?;
        // Tempo apurado (dias efetivos / 30) antes e depois de 13/11/2019.
        // A contagem de competências fica só como fallback quando não há vínculos.
        let (meses_ate_reforma, meses_apos_reforma) = match tempo.as_ref() {
            Some(t) => (t.marcos.ate_ec103 / 30, t.marcos.apos_ec103 / 30),
            None => {
                let ate = competencias
                    .iter()
                    .filter(|(c, _)| *c <= COMPETENCIA_REFORMA)
                    .count() as i64;
                (ate, quantidade as i64 - ate)
            }
        };
        let resultado = build_transicoes(&TransicoesEntrada {
            sexo: entrada.sexo.clone(),
            data_nascimento: nascimento,
            data_referencia,
            meses_ate_reforma,
            meses_apos_reforma,
            dias_totais: tempo.as_ref().map(|t| t.total.dias as i64).unwrap_or(0),
            tempo_total_360: tempo.as_ref().map(|t| t.total_360).unwrap_or(0),
            salario_beneficio,
            teto: teto_derivado,
            piso: piso_derivado,
            expectativa_sobrevida: entrada.opcoes.expectativa_sobrevida,
            filiado_antes_reforma: inicio_filiacao.and_then(|d| {
                IsoDate::parse(crate::REFORMA_EC103, "reforma")
                    .ok()
                    .map(|ref_d| d < ref_d)
            }),
            rmi_direito_adquirido,
            obs_direito_adquirido: obs_direito_adquirido.clone(),
        })?;
        if let (Some(id), Some(rmi)) = (resultado.regra_escolhida, resultado.rmi_escolhida.as_ref()) {
            if let Ok(valor) = parse_money(rmi, "RMI da regra escolhida") {
                rmi_exibicao = valor;
                regra_exibicao = Some(id.to_string());
                if let Some(regra) = resultado.regras.iter().find(|r| r.id == id) {
                    regra_nome_exibicao = Some(regra.nome.clone());
                    if matches!(
                        regra.criterio_rmi,
                        "fator-previdenciario" | "100%-media" | "direito-adquirido-reajustado"
                    ) {
                        coeficiente_exibicao = Decimal::from(100);
                    } else if regra.criterio_rmi == "coeficiente-progressivo" {
                        coeficiente_exibicao = crate::rmi::coeficiente_progressivo(
                            &entrada.sexo,
                            (meses_ate_reforma + meses_apos_reforma).max(0) as u32,
                        );
                    }
                    if regra.criterio_rmi == "fator-previdenciario" && salario_beneficio > Decimal::ZERO {
                        fator_exibicao = Some(div(rmi_exibicao, salario_beneficio)?);
                    }
                }
            }
        }
        transicoes = Some(serde_json::to_value(&resultado).expect("serialização de transições"));
    } else if especie == Especie::TempoContribuicao && !pre_reforma_dib {
        avisos.push(
            "Sem data de nascimento informada, as regras de transição da EC 103 não são comparadas.".into(),
        );
    }

    // Memória auditável.
    let divisor = divisor_usado(quantidade, entrada.opcoes.divisor_minimo);
    let soma_total: Decimal = competencias.iter().map(|(_, s)| *s).fold(Decimal::ZERO, add);
    let mut memoria = vec![MemoriaItem {
        tipo: "media",
        descricao: match &correcao {
            Some(pbc) => format!(
                "Média dos {} maiores salários corrigidos (PBC de {}; {} descartados)",
                pbc.consideradas, pbc.pbc, pbc.descartadas
            ),
            None => format!("Soma de {quantidade} competências dividida por {divisor}"),
        },
        valor: money(salario_beneficio),
    }];
    memoria.push(MemoriaItem {
        tipo: "coeficiente",
        descricao: if let Some(nome) = &regra_nome_exibicao {
            if pre_reforma_dib {
                format!("Regra vigente na DIB: {nome}")
            } else {
                format!("Regra de transição escolhida: {nome}")
            }
        } else {
            match especie {
                Especie::TempoContribuicao if pre_reforma_dib => {
                    "100% do salário de benefício × fator previdenciário (regra pré-EC 103/2019)".into()
                }
                Especie::TempoContribuicao => format!(
                    "60% + 2% × {excedente} ano(s) excedente(s) a {base} (por tempo de contribuição apurado)"
                ),
                Especie::Idade | Especie::IncapacidadePermanente | Especie::Especial | Especie::Professor => format!(
                    "Coeficiente da espécie {}: {coeficiente}% sobre a média",
                    especie.codigo()
                ),
                Especie::IncapacidadeTemporaria => {
                    "91% do salário de benefício (Lei 8.213/1991, art. 61)".into()
                }
            }
        },
        valor: format!("{coeficiente_exibicao}%"),
    });
    if let Some(fator) = fator_exibicao {
        memoria.push(MemoriaItem {
            tipo: "fator",
            descricao: if pre_reforma_dib {
                "Fator previdenciário apurado na DIB (Lei 9.876/1999 / Lei 13.183/2015)".into()
            } else {
                "Fator previdenciário aplicado pela regra vencedora (RMI ÷ salário de benefício)".into()
            },
            valor: format!("{}", fator.round_dp(4)),
        });
    }
    memoria.push(MemoriaItem {
        tipo: "rmi",
        descricao: if let Some(nome) = &regra_nome_exibicao {
            format!("RMI final pela regra escolhida: {nome}")
        } else {
            "Salário de benefício × coeficiente".into()
        },
        valor: money(rmi_exibicao),
    });
    if let Some(limite) = limite_aplicado {
        memoria.push(MemoriaItem {
            tipo: "limite",
            descricao: match limite {
                "teto" => "Teto aplicado".into(),
                "piso" => "Piso aplicado".into(),
                "media_12" => {
                    "Limite da média dos últimos 12 salários (Lei 8.213/1991, art. 29, § 10)"
                        .into()
                }
                outro => format!("Limite {outro} aplicado"),
            },
            valor: money(rmi_bruta),
        });
    }

    // Avisos (paridade com o implementação de referência + extensões deste motor).
    avisos.push(if correcao_ativa {
        match entrada
            .opcoes
            .perfil_correcao
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(perfil_id) => format!(
                "Salários nominais corrigidos pelo perfil de correção {perfil_id} (séries por faixa de vigência) com o teto da época aplicado antes da correção."
            ),
            None => {
                "Salários nominais corrigidos pelo INPC com o teto da época aplicado antes da correção.".into()
            }
        }
    } else {
        "Informe os salários de contribuição já corrigidos para a mesma data de referência; a correção monetária é etapa separada.".into()
    });
    if correcao_ativa {
        if let Some(correcao) = &correcao {
            if !correcao.substituicoes_inpc.is_empty() {
                avisos.push(format!(
                    "Fallback documentado pelo INPC: {} mês(es) sem série oficial carregada ({}) foram corrigidos pelo INPC; o resultado não reproduz a correção oficial enquanto as séries não forem carregadas.",
                    correcao.substituicoes_inpc.len(),
                    correcao.substituicoes_inpc.join(", ")
                ));
            }
            if !correcao.series_ausentes.is_empty() {
                avisos.push(format!(
                    "Séries oficiais ausentes do catálogo de correção: {}. Carregue os arquivos versionados (JSON/CSV) para a correção oficial.",
                    correcao.series_ausentes.join(", ")
                ));
            }
        }
    }
    avisos.push(
        "Tempo especial, rural e concomitância exigem análise própria; confira o CNIS e a carta de concessão antes de usar o resultado.".into(),
    );
    if correcao_ativa {
        if let Some(ref_ef) = correcao.as_ref().and_then(|pbc| pbc.referencia_efetiva.as_deref()) {
            avisos.push(format!(
                "Correção monetária acumulada até o último índice oficial publicado ({ref_ef}); competências posteriores utilizam fator 1,0000."
            ));
        }
        if let Some(minimo) = entrada.opcoes.descartar_menores_minimo {
            let descartadas = correcao.as_ref().map(|pbc| pbc.descartadas).unwrap_or(0);
            avisos.push(format!(
                "Descarte mínimo da EC 103/2019, art. 26, § 6º, aplicado: {descartadas} menor(es) remuneração(ões) excluída(s) do PBC, mantendo {minimo} competência(s); tempo e carência foram recalculados."
            ));
        } else if entrada.opcoes.descarte_otimo && !pre_reforma_dib {
            let descartadas = correcao.as_ref().map(|pbc| pbc.descartadas).unwrap_or(0);
            avisos.push(format!(
                "Descarte ótimo automático da EC 103/2019, art. 26, § 6º, aplicado: {descartadas} menor(es) competência(s) excluída(s) para maximizar a RMI mantendo tempo e carência."
            ));
        } else if entrada.opcoes.descarte || (pre_reforma_dib && entrada.opcoes.descarte_otimo) {
            avisos.push("Descarte fixo dos 20% menores salários corrigidos selecionado.".into());
        } else {
            avisos.push("Nenhum descarte de salários selecionado.".into());
        }
        avisos.push(
            "Competências anteriores a 2001 podem divergir da tabela de transição oficial do INSS; o descarte costuma absorver a diferença.".into(),
        );
    }
    if especie == Especie::TempoContribuicao && !pre_reforma_dib && anos < base {
        avisos.push(format!(
            "Tempo informado ({anos} anos) abaixo do mínimo de {base} anos para o coeficiente cheio; o coeficiente permanece em 60%."
        ));
    }
    if entrada.opcoes.divisor_minimo && (quantidade as u32) < crate::DIVISOR_MINIMO {
        avisos.push(format!(
            "Divisor mínimo aplicado: a soma foi dividida por {} (Lei 14.331/2022), e não por {quantidade} competências.",
            crate::DIVISOR_MINIMO
        ));
    }
    if entrada.opcoes.aplicar_teto_vigente_der {
        avisos.push(
            "Teto vigente do salário de contribuição aplicado automaticamente na competência de referência.".into(),
        );
    }
    if especie != Especie::TempoContribuicao {
        let regra_ref = avaliacao
            .regras
            .iter()
            .find(|r| Some(r.id) == avaliacao.regra_escolhida)
            .or_else(|| avaliacao.regras.iter().min_by_key(|r| r.faltas_meses));
        memoria.push(MemoriaItem {
            tipo: "especie",
            descricao: format!("Espécie {} — {}", avaliacao.especie, avaliacao.nome),
            valor: regra_ref
                .map(|r| r.nome.clone())
                .unwrap_or_else(|| "requisitos não cumpridos".into()),
        });
        if let Some(regra) = regra_ref {
            for req in &regra.requisitos {
                memoria.push(MemoriaItem {
                    tipo: "requisito",
                    descricao: format!(
                        "{} — {}: {} (exigido: {}); {}",
                        regra.nome,
                        req.descricao,
                        req.apurado,
                        req.exigido,
                        if req.cumprido { "cumprido" } else { "não cumprido" }
                    ),
                    valor: req
                        .falta_meses
                        .filter(|m| *m > 0)
                        .map(|m| format!("faltam {m} meses"))
                        .unwrap_or_default(),
                });
            }
        }
        for observacao in &avaliacao.observacoes {
            avisos.push(observacao.clone());
        }
        if avaliacao.regra_escolhida.is_none() {
            avisos.push(
                "Requisitos da espécie não cumpridos na DER. O valor calculado é uma simulação condicional, não uma concessão."
                    .into(),
            );
        }
    }

    let coeficiente_str = format!("{coeficiente}");
    Ok(RgpsResultado {
        segurado: SeguradoView {
            nome: entrada.nome.clone(),
            sexo: entrada.sexo.clone(),
            data_nascimento: entrada.data_nascimento.map(|d| d.to_string()),
        },
        especie: EspecieView {
            codigo: if especie == Especie::TempoContribuicao {
                "42".into()
            } else {
                avaliacao.especie.to_string()
            },
            nome: if especie == Especie::TempoContribuicao {
                "Aposentadoria por tempo de contribuição".into()
            } else {
                avaliacao.nome.to_string()
            },
            regra_escolhida: if pre_reforma_dib {
                regra_exibicao.clone()
            } else if especie == Especie::TempoContribuicao {
                transicoes
                    .as_ref()
                    .and_then(|t| t.get("regra_escolhida"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            } else {
                avaliacao.regra_escolhida.map(str::to_string)
            },
            regra_nome: if pre_reforma_dib {
                regra_nome_exibicao.clone()
            } else if especie == Especie::TempoContribuicao {
                regra_nome_exibicao.clone()
            } else {
                avaliacao
                    .regras
                    .iter()
                    .find(|r| Some(r.id) == avaliacao.regra_escolhida)
                    .map(|r| r.nome.clone())
            },
            elegivel: if pre_reforma_dib {
                pre_reforma_elegivel
            } else if especie == Especie::TempoContribuicao {
                transicoes
                    .as_ref()
                    .and_then(|t| t.get("regra_escolhida"))
                    .is_some()
            } else {
                avaliacao.regra_escolhida.is_some()
            },
        },
        primeira_competencia: primeira.to_string(),
        ultima_competencia: ultima.to_string(),
        parametros: ParametrosView {
            divisor: correcao
                .as_ref()
                .map(|pbc| pbc.divisor_usado)
                .filter(|d| *d > 0)
                .unwrap_or(divisor),
            divisor_minimo_aplicado: entrada.opcoes.divisor_minimo
                && correcao
                    .as_ref()
                    .map(|pbc| pbc.consideradas)
                    .unwrap_or(quantidade)
                    < crate::DIVISOR_MINIMO as usize,
            anos,
            meses_restantes,
            anos_base: base,
            coeficiente: coeficiente_str.clone(),
            coeficiente_regra: (coeficiente_exibicao != coeficiente)
                .then(|| format!("{coeficiente_exibicao}")),
            correcao_ativa,
            referencia_correcao: correcao
                .as_ref()
                .map(|pbc| pbc.referencia.clone()),
            descarte_aplicado: correcao.as_ref().map(|pbc| pbc.descarte_aplicado),
            perfil_correcao: correcao.as_ref().and_then(|pbc| pbc.perfil.clone()),
            fallback_inpc_documentado: entrada.opcoes.fallback_inpc_documentado,
        },
        resumo: ResumoView {
            competencias: quantidade,
            carencia_meses: carencia.carencia_meses,
            soma_salarios: match &correcao {
                Some(pbc) => pbc.soma_considerada.clone(),
                None => money(soma_total),
            },
            salario_beneficio: money(salario_beneficio),
            coeficiente: format!("{coeficiente_exibicao}"),
            rmi_bruta: money(rmi_bruta),
            rmi: money(rmi_exibicao),
            limite_aplicado,
            teto: teto_derivado.map(money),
            piso: piso_derivado.map(money),
            regra_escolhida: regra_exibicao,
            regra_nome: regra_nome_exibicao,
        },
        tempo: tempo.map(|t| serde_json::to_value(&t).expect("serialização de tempo")),
        carencia: serde_json::to_value(&carencia).expect("serialização de carência"),
        correcao: correcao
            .as_ref()
            .map(|pbc| serde_json::to_value(pbc).expect("serialização de PBC")),
        memoria,
        transicoes,
        beneficio: if especie == Especie::TempoContribuicao {
            None
        } else {
            Some(serde_json::to_value(&avaliacao).expect("serialização do benefício"))
        },
        simulacao_descartes,
        avisos,
    })
}

fn calcular_direito_adquirido(
    entrada: &RgpsEntrada,
    especie: Especie,
    competencias: &[(Competencia, Decimal)],
    tempo: Option<&crate::tempo::TempoResultado>,
    inicio_filiacao: Option<IsoDate>,
    perfil_cfg: Option<(
        &crate::perfil_correcao::CatalogoSeries,
        &crate::perfil_correcao::PerfilCorrecao,
        crate::perfil_correcao::PoliticaFallback,
    )>,
) -> (Option<Decimal>, Option<String>) {
    if especie != Especie::TempoContribuicao {
        return (None, None);
    }
    let Some(data_ref) = entrada.data_referencia else {
        return (None, None);
    };
    let Ok(reforma_dt) = IsoDate::parse(crate::REFORMA_EC103, "EC103") else {
        return (None, None);
    };
    if data_ref < reforma_dt {
        return (None, None);
    }
    let Some(nascimento) = entrada.data_nascimento else {
        return (None, None);
    };
    let ate_reforma_meses = match tempo {
        Some(t) => t.marcos.ate_ec103 / 30,
        None => competencias.iter().filter(|(c, _)| *c <= COMPETENCIA_REFORMA).count() as i64,
    };
    let tempo_min = if entrada.sexo == "feminino" { 360 } else { 420 };
    if ate_reforma_meses < tempo_min {
        return (None, None);
    }
    let pre_ec_comps: Vec<(Competencia, Decimal)> = competencias
        .iter()
        .copied()
        .filter(|(c, _)| *c < COMPETENCIA_REFORMA)
        .collect();
    if pre_ec_comps.is_empty() {
        return (None, None);
    }
    let ref_pre = Competencia { ano: 2019, mes: 11 };
    let Ok((mut parcelas_pre, _, _, _)) =
        corrigir_parcelas_opcoes(&pre_ec_comps, &ref_pre, true, perfil_cfg)
    else {
        return (None, None);
    };
    parcelas_pre.sort_by(|a, b| {
        b.corrigido_full
            .partial_cmp(&a.corrigido_full)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let consideradas = ((parcelas_pre.len() as f64) * 0.8).floor().max(1.0) as usize;
    let soma = parcelas_pre[..consideradas]
        .iter()
        .map(|p| to_cents(p.corrigido_full))
        .fold(Decimal::ZERO, add);
    let divisor_pre = {
        let pbc_ini = Competencia { ano: 1994, mes: 7 };
        let inicio_fil_comp = inicio_filiacao.map(|d| Competencia {
            ano: d.ano as u16,
            mes: d.mes,
        });
        let inicio_periodo = inicio_fil_comp
            .or_else(|| pre_ec_comps.first().map(|(c, _)| *c))
            .map(|c| c.max(pbc_ini))
            .unwrap_or(pbc_ini);
        let fim_periodo = Competencia { ano: 2019, mes: 10 };
        let decorridos = if inicio_periodo <= fim_periodo {
            inicio_periodo.meses_ate(&fim_periodo).max(1) as usize
        } else {
            consideradas
        };
        let min_60 = ((decorridos as f64) * 0.6).floor() as usize;
        consideradas.max(min_60).max(1)
    };
    let Ok(sb_pre) = div(soma, Decimal::from(divisor_pre as u64)).map(to_cents) else {
        return (None, None);
    };
    let idade_pre_meses = idade_em_meses(&nascimento, &reforma_dt);
    let idade_pre_anos = (idade_pre_meses / 12).clamp(0, 90) as usize;
    let es_pre = crate::dados::expectativa_sobrevida::EXPECTATIVA_GERAL
        .iter()
        .filter(|(v, _)| *v <= crate::REFORMA_EC103)
        .map(|(_, row)| row)
        .last()
        .and_then(|row| Decimal::from_str_exact(row[idade_pre_anos]).ok());
    let Some(es) = es_pre else {
        return (None, None);
    };
    let tempo_pre_dec = Decimal::from(tempo.map(|t| t.marcos.ate_ec103).unwrap_or(ate_reforma_meses * 30)) / Decimal::from(30);
    let Ok(fator_bruto) = fator_previdenciario(&entrada.sexo, tempo_pre_dec, idade_pre_meses, es) else {
        return (None, None);
    };
    let fator_trunc = fator_bruto.trunc_with_scale(4);
    let pontos_pre = ate_reforma_meses + idade_pre_meses;
    let meta_pontos_pre = if entrada.sexo == "feminino" { 86 * 12 } else { 96 * 12 };
    let (fator_efetivo, afastado_pontos) = if pontos_pre >= meta_pontos_pre && fator_trunc < Decimal::ONE {
        (Decimal::ONE, true)
    } else {
        (fator_trunc, false)
    };
    let mut rmi_2019 = to_cents(mul(sb_pre, fator_efetivo));
    if let Ok(teto_19) = teto_vigente(&ref_pre) {
        if rmi_2019 > teto_19 {
            rmi_2019 = teto_19;
        }
    }
    match crate::dados::reajustes_beneficios::reajustar_renda_manutencao(rmi_2019, &reforma_dt, &data_ref) {
        Some((rmi_dib, etapas)) => {
            let detalhe_fator = if afastado_pontos {
                "fator afastado pela regra 86/96 pontos".to_string()
            } else {
                format!("fator pre-EC {:.4}", fator_efetivo)
            };
            let obs = format!(
                "Direito adquirido em 13/11/2019: SB R$ {} ({detalhe_fator}), RMI base R$ {} reajustada em {} etapa(s) até a DIB para R$ {}.",
                money(sb_pre),
                money(rmi_2019),
                etapas.len(),
                money(rmi_dib),
            );
            (Some(rmi_dib), Some(obs))
        }
        None => (
            None,
            Some("Direito adquirido atendido em 13/11/2019; reajuste automático indisponível para o intervalo anual até esta DIB.".into()),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn simular_descarte_otimo(
    entrada: &RgpsEntrada,
    especie: Especie,
    competencias: &[(Competencia, Decimal)],
    parcelas: &[ParcelaCorrigida],
    referencia_correcao: &Competencia,
    tempo: Option<&crate::tempo::TempoResultado>,
    carencia_base: usize,
    inicio_filiacao: Option<IsoDate>,
    rural: bool,
    teto_derivado: Option<Decimal>,
    piso_derivado: Option<Decimal>,
    referencia_efetiva: Option<String>,
    rmi_direito_adquirido: Option<Decimal>,
    obs_direito_adquirido: Option<&str>,
    simulacao_out: &mut Vec<SimulacaoDescarteView>,
) -> Result<PbcResultado> {
    if parcelas.is_empty() {
        return crate::money::erro("Descarte exige ao menos uma competência no PBC.");
    }
    let mut ordenadas: Vec<&ParcelaCorrigida> = parcelas.iter().collect();
    ordenadas.sort_by(|a, b| {
        a.corrigido_full
            .partial_cmp(&b.corrigido_full)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.competencia.cmp(&b.competencia))
    });

    let base_360 = tempo.map(|t| t.total_360).unwrap_or((competencias.len() as i64) * 30);
    let base_ate = tempo
        .map(|t| t.marcos.ate_ec103)
        .unwrap_or_else(|| (competencias.iter().filter(|(c, _)| *c <= COMPETENCIA_REFORMA).count() as i64) * 30);
    let base_apos = tempo
        .map(|t| t.marcos.apos_ec103)
        .unwrap_or_else(|| (competencias.iter().filter(|(c, _)| *c > COMPETENCIA_REFORMA).count() as i64) * 30);
    let soma_total_cent: Decimal = parcelas
        .iter()
        .map(|p| to_cents(p.corrigido_full))
        .fold(Decimal::ZERO, add);

    let filiado_pre = inicio_filiacao.and_then(|d| {
        IsoDate::parse(crate::REFORMA_EC103, "reforma")
            .ok()
            .map(|r| d < r)
    });

    let avaliar_n = |n: usize, soma_rem: Decimal, ate_360: i64, apos_360: i64, total_360: i64, rem_car: usize| -> Result<(bool, Option<String>, Decimal, Decimal)> {
        let consideradas = parcelas.len() - n;
        let divisor = divisor_usado(consideradas, entrada.opcoes.divisor_minimo);
        let sb = to_cents(div(soma_rem, Decimal::from(divisor))?);
        if especie == Especie::TempoContribuicao {
            if let (Some(nasc), Some(der)) = (entrada.data_nascimento, entrada.data_referencia) {
                let res = build_transicoes(&TransicoesEntrada {
                    sexo: entrada.sexo.clone(),
                    data_nascimento: nasc,
                    data_referencia: der,
                    meses_ate_reforma: ate_360 / 30,
                    meses_apos_reforma: apos_360 / 30,
                    dias_totais: total_360 % 30,
                    tempo_total_360: total_360,
                    salario_beneficio: sb,
                    teto: teto_derivado,
                    piso: piso_derivado,
                    expectativa_sobrevida: entrada.opcoes.expectativa_sobrevida,
                    filiado_antes_reforma: filiado_pre,
                    rmi_direito_adquirido: if n == 0 { rmi_direito_adquirido } else { None },
                    obs_direito_adquirido: obs_direito_adquirido.map(str::to_string),
                })?;
                if let (Some(id), Some(rmi_s)) = (res.regra_escolhida, res.rmi_escolhida.as_deref()) {
                    let rmi_val = parse_money(rmi_s, "rmi")?;
                    return Ok((true, Some(id.to_string()), sb, rmi_val));
                }
                return Ok((false, None, sb, Decimal::ZERO));
            }
        }
        let media_12 = {
            let mut sorted_chron: Vec<&ParcelaCorrigida> = ordenadas[n..].to_vec();
            sorted_chron.sort_by_key(|p| p.competencia);
            let last12: Vec<Decimal> = sorted_chron.iter().rev().take(12).map(|p| to_cents(p.corrigido_full)).collect();
            if last12.is_empty() { None } else { div(last12.iter().copied().fold(Decimal::ZERO, add), Decimal::from(last12.len() as u64)).ok() }
        };
        let eval = especie::avaliar(&especie::AvaliacaoEntrada {
            especie,
            sexo: &entrada.sexo,
            data_nascimento: entrada.data_nascimento,
            data_referencia: entrada.data_referencia,
            carencia_meses: rem_car as i64,
            tempo_meses: (ate_360 + apos_360) / 30,
            competencias: consideradas as i64,
            inicio_filiacao,
            rural,
            incapacidade_origem: entrada.opcoes.incapacidade_origem.as_deref(),
            media_ultimos_12: media_12,
            especial_grau: entrada.opcoes.especial_grau.as_deref(),
            especial_meses: entrada.opcoes.tempo_especial_meses,
            especial_antes_reforma: entrada.opcoes.especial_antes_reforma,
            magisterio_meses: entrada.opcoes.magisterio_meses,
            magisterio_exclusivo: entrada.opcoes.magisterio_exclusivo,
            magisterio_antes_reforma: entrada.opcoes.magisterio_antes_reforma,
            tempo_ate_reforma_meses: ate_360 / 30,
        })?;
        let mut rmi_v = to_cents(mul(sb, div(eval.coeficiente, Decimal::from(100))?));
        if let Some(teto) = teto_derivado {
            if rmi_v > teto { rmi_v = teto; }
        }
        if let Some(piso) = piso_derivado {
            if rmi_v < piso { rmi_v = piso; }
        }
        Ok((eval.regra_escolhida.is_some(), eval.regra_escolhida.map(str::to_string), sb, rmi_v))
    };

    let mut soma_rem = soma_total_cent;
    let mut ate_360 = base_ate;
    let mut apos_360 = base_apos;
    let mut total_360 = base_360;
    let mut rem_car = carencia_base;

    let (init_ok, init_regra, init_sb, init_rmi) = avaliar_n(0, soma_rem, ate_360, apos_360, total_360, rem_car)?;
    simulacao_out.push(SimulacaoDescarteView {
        n: 0,
        competencia_removida: None,
        valor_removido: None,
        tempo_meses: total_360 / 30,
        carencia_meses: rem_car,
        divisor: divisor_usado(parcelas.len(), entrada.opcoes.divisor_minimo),
        salario_beneficio: money(init_sb),
        regra_escolhida: init_regra.clone(),
        rmi: init_ok.then(|| money(init_rmi)),
        aceita: init_ok,
    });

    if !init_ok {
        return pbc_de_parcelas_excluindo(
            parcelas,
            referencia_correcao,
            &BTreeSet::new(),
            entrada.opcoes.divisor_minimo,
            referencia_efetiva,
        );
    }

    let mut best_n = 0usize;
    let mut best_rmi = init_rmi;
    let max_disc = if entrada.opcoes.divisor_minimo && parcelas.len() <= crate::DIVISOR_MINIMO as usize {
        0
    } else if entrada.opcoes.divisor_minimo {
        parcelas.len().saturating_sub(crate::DIVISOR_MINIMO as usize)
    } else {
        parcelas.len().saturating_sub(1)
    };

    for n in 1..=max_disc {
        let p = ordenadas[n - 1];
        let val = to_cents(p.corrigido_full);
        soma_rem = soma_rem - val;
        total_360 = (total_360 - 30).max(0);
        if p.competencia <= COMPETENCIA_REFORMA {
            ate_360 = (ate_360 - 30).max(0);
        } else {
            apos_360 = (apos_360 - 30).max(0);
        }
        rem_car = rem_car.saturating_sub(1);

        let (ok, regra, sb, rmi) = avaliar_n(n, soma_rem, ate_360, apos_360, total_360, rem_car)?;
        let melhorou = ok && rmi > best_rmi;
        if melhorou {
            best_rmi = rmi;
            best_n = n;
        }
        if parcelas.len() <= 60 || melhorou || !ok {
            simulacao_out.push(SimulacaoDescarteView {
                n,
                competencia_removida: Some(p.competencia.to_string()),
                valor_removido: Some(money(val)),
                tempo_meses: total_360 / 30,
                carencia_meses: rem_car,
                divisor: divisor_usado(parcelas.len() - n, entrada.opcoes.divisor_minimo),
                salario_beneficio: money(sb),
                regra_escolhida: regra,
                rmi: ok.then(|| money(rmi)),
                aceita: ok,
            });
        }
        if !ok {
            break;
        }
    }

    let descartadas_set: BTreeSet<Competencia> = ordenadas[..best_n]
        .iter()
        .map(|p| p.competencia)
        .collect();
    pbc_de_parcelas_excluindo(
        parcelas,
        referencia_correcao,
        &descartadas_set,
        entrada.opcoes.divisor_minimo,
        referencia_efetiva,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::parse_money;

    fn competencia(v: &str) -> Competencia {
        Competencia::parse(v, "t").unwrap()
    }

    fn entrada_feminina_300() -> RgpsEntrada {
        RgpsEntrada {
            nome: Some("Cliente Teste".into()),
            especie: None,
            sexo: "feminino".into(),
            data_nascimento: None,
            data_referencia: None,
            vinculos: vec![],
            competencias: (0..300)
                .map(|i| {
                    let total = 1994 * 12 + 6 + i as u16;
                    (
                        Competencia {
                            ano: total / 12,
                            mes: (total % 12 + 1) as u8,
                        },
                        parse_money("2000.00", "s").unwrap(),
                    )
                })
                .collect(),
            opcoes: RgpsOpcoes::default(),
        }
    }

    #[test]
    fn especie_sem_motor_bloqueia() {
        let mut entrada = entrada_feminina_300();
        entrada.especie = Some("80".into());
        let erro = calcular(&entrada).unwrap_err();
        assert!(erro.0.contains("não tem fórmula"), "{erro}");
        entrada.especie = Some("42".into());
        assert!(calcular(&entrada).is_ok());
    }

    #[test]
    fn descarte_ec103_recalcula_tempo_e_competencias() {
        let mut entrada = entrada_feminina_300();
        entrada.competencias = (0..10)
            .map(|i| {
                (
                    Competencia { ano: 2024, mes: (i + 1) as u8 },
                    parse_money(&format!("{}.00", (i + 1) * 100), "s").unwrap(),
                )
            })
            .collect();
        entrada.vinculos = vec![VinculoEntrada {
            inicio: IsoDate::parse("2024-01-01", "inicio").unwrap(),
            fim: Some(IsoDate::parse("2024-12-31", "fim").unwrap()),
            tipo: "empregado".into(),
            empresa: None,
            fator: 1.0,
        }];
        entrada.opcoes.corrigir_salarios = true;
        entrada.opcoes.referencia_correcao = Some(Competencia { ano: 2024, mes: 10 });
        entrada.opcoes.descartar_menores_minimo = Some(8);
        let resultado = calcular(&entrada).unwrap();
        let correcao = resultado.correcao.as_ref().unwrap();
        assert_eq!(correcao["pbc"], 10);
        assert_eq!(correcao["descartadas"], 2);
        assert_eq!(correcao["consideradas"], 8);
        let tempo = resultado.tempo.as_ref().unwrap();
        assert!(tempo["avisos"].as_array().unwrap().iter().any(|a| a.as_str().unwrap_or("").contains("descartada(s)")));
    }
    #[test]
    fn reform_split_usa_tempo_apurado_e_nao_competencias() {
        let mut entrada = entrada_feminina_300();
        entrada.data_nascimento = Some(IsoDate::parse("1960-01-01", "nascimento").unwrap());
        entrada.data_referencia = Some(IsoDate::parse("2025-01-31", "referencia").unwrap());
        entrada.vinculos = vec![VinculoEntrada {
            inicio: IsoDate::parse("2019-01-01", "inicio").unwrap(),
            fim: Some(IsoDate::parse("2019-11-13", "fim").unwrap()),
            tipo: "empregado".into(),
            empresa: None,
            fator: 1.0,
        }];
        let resultado = calcular(&entrada).unwrap();
        let transicoes = resultado.transicoes.unwrap();
        // 2019-01-01..2019-11-13 = 317 dias inclusivos; 317/30 = 10 meses.
        assert_eq!(transicoes["tempo_ate_reforma_meses"], 10);
        assert_eq!(transicoes["tempo_total_meses"], 10);
    }

    #[test]
    fn caso_ancora_mulher_300_competencias() {
        let resultado = calcular(&entrada_feminina_300()).unwrap();
        assert_eq!(resultado.resumo.salario_beneficio, "2000.00");
        assert_eq!(resultado.resumo.coeficiente, "80");
        assert_eq!(resultado.resumo.rmi, "1600.00");
        assert_eq!(resultado.resumo.carencia_meses, 300);
    }

    #[test]
    fn rejeita_duplicada_e_pre_1994_07() {
        let mut entrada = entrada_feminina_300();
        entrada.competencias.truncate(2);
        entrada.competencias[1].0 = entrada.competencias[0].0;
        assert!(calcular(&entrada).is_err());

        let mut entrada = entrada_feminina_300();
        entrada.competencias.clear();
        entrada.competencias.push((competencia("1994-06"), parse_money("100", "s").unwrap()));
        assert!(calcular(&entrada).is_err());
    }

    #[test]
    fn divisor_minimo_50_competencias() {
        let mut entrada = entrada_feminina_300();
        entrada.competencias.truncate(50);
        entrada.opcoes.divisor_minimo = true;
        let resultado = calcular(&entrada).unwrap();
        // 50 × 2000 = 100.000 / 108 = 925.9259... → 925.93; coeficiente 60%; RMI 555.56 (925.93×0.6 = 555.5555...)
        assert_eq!(resultado.resumo.salario_beneficio, "925.93");
        assert_eq!(resultado.resumo.coeficiente, "60");
        assert_eq!(resultado.resumo.rmi, "555.56");
    }

    #[test]
    fn aplica_piso() {
        let mut entrada = entrada_feminina_300();
        entrada.competencias.truncate(12);
        entrada.opcoes.piso = Some(parse_money("1518.00", "p").unwrap());
        let resultado = calcular(&entrada).unwrap();
        // SB 2000 × 60% = 1200 < 1518 → piso.
        assert_eq!(resultado.resumo.limite_aplicado, Some("piso"));
        assert_eq!(resultado.resumo.rmi, "1518.00");
    }

    #[test]
    fn perfil_sem_correcao_ativa_falha_no_nucleo_e_nao_e_ignorado() {
        // Caminho que antes passava em silêncio: perfil informado, correção
        // desligada, cálculo devolvia Ok como se o perfil não existisse.
        let mut entrada = entrada_feminina_300();
        entrada.opcoes.perfil_correcao = Some("trf3-v1".into());
        entrada.opcoes.corrigir_salarios = false;
        let erro = calcular(&entrada).expect_err("perfil com correção desligada deve bloquear");
        assert!(erro.0.contains("corrigir_salarios=false"), "{erro}");
        assert!(erro.0.contains("trf3-v1"), "{erro}");
        // A porta pública de validação também bloqueia, não só calcular().
        assert!(validar(&entrada).is_err());
    }

    #[test]
    fn perfil_desconhecido_falha_na_validacao_antes_do_calculo() {
        let mut entrada = entrada_feminina_300();
        entrada.opcoes.perfil_correcao = Some("perfil-inexistente".into());
        entrada.opcoes.corrigir_salarios = true;
        let erro = validar(&entrada).unwrap_err();
        assert!(erro.0.contains("trf3-v1"), "lista os disponíveis: {erro}");
        assert!(calcular(&entrada).is_err());
    }

    #[test]
    fn fallback_sem_perfil_falha_na_validacao() {
        let mut entrada = entrada_feminina_300();
        entrada.opcoes.fallback_inpc_documentado = true;
        let erro = validar(&entrada).unwrap_err();
        assert!(erro.0.contains("exige perfil de correção"), "{erro}");
    }

    #[test]
    fn perfil_vazio_ou_com_espacos_nao_arma_a_guardas() {
        // Campo em branco é ausência declarada, não perfil: não bloqueia.
        let mut entrada = entrada_feminina_300();
        entrada.opcoes.perfil_correcao = Some("   ".into());
        entrada.opcoes.corrigir_salarios = false;
        assert!(validar(&entrada).is_ok());
        assert_eq!(calcular(&entrada).unwrap().resumo.rmi, "1600.00");
    }
    #[test]
    fn carencia_conta_contribuicoes_e_nao_meses_de_vinculo() {
        let mut entrada = entrada_feminina_300();
        entrada.especie = Some("42".into());
        entrada.vinculos = vec![VinculoEntrada {
            inicio: IsoDate::parse("2024-01-01", "inicio").unwrap(),
            fim: Some(IsoDate::parse("2024-12-31", "fim").unwrap()),
            tipo: "empregado".into(),
            empresa: None,
            fator: 1.0,
        }];
        entrada.competencias = vec![
            (Competencia { ano: 2024, mes: 1 }, parse_money("2000.00", "s").unwrap()),
            (Competencia { ano: 2024, mes: 2 }, parse_money("2000.00", "s").unwrap()),
        ];
        entrada.opcoes.meses_fora_do_tempo = vec![
            Competencia { ano: 2024, mes: 3 },
            Competencia { ano: 2024, mes: 4 },
        ];
        let resultado = calcular(&entrada).unwrap();
        assert_eq!(resultado.resumo.carencia_meses, 4);
        assert_eq!(resultado.carencia.get("competencias_vinculo").and_then(|v| v.as_u64()), Some(0));
    }

    #[test]
    fn referencia_limites_honra_data_referencia_quando_maior_que_ultima_competencia() {
        let competencias: Vec<(Competencia, Decimal)> = (1..=12u8)
            .map(|m| {
                (
                    Competencia { ano: 2024, mes: m },
                    parse_money("10000.00", "s").unwrap(),
                )
            })
            .collect();
        let entrada = RgpsEntrada {
            nome: None,
            especie: Some("41".into()),
            sexo: "feminino".into(),
            data_nascimento: Some(IsoDate::parse("1960-01-01", "n").unwrap()),
            data_referencia: Some(IsoDate::parse("2025-05-10", "dib").unwrap()),
            vinculos: vec![],
            competencias,
            opcoes: RgpsOpcoes {
                corrigir_salarios: false,
                aplicar_teto_vigente_der: true,
                aplicar_piso_automatico: true,
                ..RgpsOpcoes::default()
            },
        };
        let resultado = calcular(&entrada).unwrap();
        assert_eq!(resultado.resumo.teto.as_deref(), Some("8157.41"));
        assert_eq!(resultado.resumo.piso.as_deref(), Some("1518.00"));
    }

    #[test]
    fn dib_pre_ec103_especie_42_aplica_fator_pontos_e_ignora_60_mais_2_e_transicoes() {
        let mut comps_ate_2018_05 = Vec::new();
        for idx in 0..287u16 {
            let total = 1994u16 * 12 + 6 + idx;
            let ano = total / 12;
            let mes = (total % 12) as u8 + 1;
            comps_ate_2018_05.push((Competencia { ano, mes }, parse_money("4000.00", "s").unwrap()));
        }
        let mut comps_ate_2019_05 = comps_ate_2018_05.clone();
        for idx in 287..299u16 {
            let total = 1994u16 * 12 + 6 + idx;
            let ano = total / 12;
            let mes = (total % 12) as u8 + 1;
            comps_ate_2019_05.push((Competencia { ano, mes }, parse_money("4000.00", "s").unwrap()));
        }

        // Caso A: DIB 2018-06-01, mulher 50 anos + 30 anos = 80 pts (< 85) -> aplica fator (< 1.0), sem 60%+2%, sem transições
        let entrada_fator = RgpsEntrada {
            nome: None,
            especie: Some("42".into()),
            sexo: "feminino".into(),
            data_nascimento: Some(IsoDate::parse("1968-01-01", "n").unwrap()),
            data_referencia: Some(IsoDate::parse("2018-06-01", "dib").unwrap()),
            vinculos: vec![VinculoEntrada {
                inicio: IsoDate::parse("1988-06-01", "i").unwrap(),
                fim: Some(IsoDate::parse("2018-05-31", "f").unwrap()),
                tipo: "empregado".into(),
                empresa: None,
                fator: 1.0,
            }],
            competencias: comps_ate_2018_05.clone(),
            opcoes: RgpsOpcoes {
                corrigir_salarios: true,
                descarte_otimo: true,
                aplicar_teto_vigente_der: true,
                aplicar_piso_automatico: true,
                ..RgpsOpcoes::default()
            },
        };
        let res_fator = calcular(&entrada_fator).unwrap();
        assert!(res_fator.transicoes.is_none());
        assert!(res_fator.simulacao_descartes.is_empty());
        assert_eq!(res_fator.parametros.coeficiente, "100");
        assert_eq!(res_fator.parametros.anos_base, 30);
        assert_eq!(res_fator.especie.regra_escolhida.as_deref(), Some("pre-ec103-fator"));
        assert!(res_fator.especie.elegivel);
        assert!(res_fator.memoria.iter().all(|m| !m.descricao.contains("60% + 2%")));
        let sb_f: Decimal = res_fator.resumo.salario_beneficio.parse().unwrap();
        let rmi_f: Decimal = res_fator.resumo.rmi.parse().unwrap();
        assert!(rmi_f < sb_f, "fator < 1 deve reduzir RMI: {rmi_f} < {sb_f}");

        // Caso B: DIB 2018-06-01, mulher 55 anos + 30 anos = 85 pts (>= 85 em Lei 13.183/2015) -> fator afastado
        let mut entrada_85 = entrada_fator.clone();
        entrada_85.data_nascimento = Some(IsoDate::parse("1963-05-01", "n").unwrap());
        let res_85 = calcular(&entrada_85).unwrap();
        assert_eq!(res_85.especie.regra_escolhida.as_deref(), Some("pre-ec103-pontos"));
        assert_eq!(res_85.resumo.rmi, res_85.resumo.salario_beneficio);

        // Caso C: DIB 2019-06-01 (>= 2018-12-31) exige 86/96 pontos; 85 pontos não afastam o fator
        let entrada_86 = RgpsEntrada {
            data_nascimento: Some(IsoDate::parse("1964-05-01", "n").unwrap()),
            data_referencia: Some(IsoDate::parse("2019-06-01", "dib").unwrap()),
            vinculos: vec![VinculoEntrada {
                inicio: IsoDate::parse("1989-06-01", "i").unwrap(),
                fim: Some(IsoDate::parse("2019-05-31", "f").unwrap()),
                tipo: "empregado".into(),
                empresa: None,
                fator: 1.0,
            }],
            competencias: comps_ate_2019_05,
            ..entrada_fator
        };
        let res_86 = calcular(&entrada_86).unwrap();
        assert_eq!(res_86.especie.regra_escolhida.as_deref(), Some("pre-ec103-fator"));
        let sb_86: Decimal = res_86.resumo.salario_beneficio.parse().unwrap();
        let rmi_86: Decimal = res_86.resumo.rmi.parse().unwrap();
        assert!(rmi_86 < sb_86, "em 2019-06 exige 86 pontos; 85 pontos sofrem fator: {rmi_86} < {sb_86}");
    }

    #[test]
    fn direito_adquirido_usa_inicio_filiacao_e_nao_primeira_competencia_pre_ec() {
        // Filiação em 1984-01-01 (> 30 anos até 2015-12-31), mas com competências apenas de 2012-01 a 2019-10 (94 meses -> 75 consideradas).
        // Divisor mínimo no direito adquirido deve partir de 1994-07 até 2019-10 (304 * 0.6 = 182), e não de 2012-01 (divisor 75).
        let mut competencias = Vec::new();
        for idx in 0..94u16 {
            let total = 2012u16 * 12 + idx;
            let ano = total / 12;
            let mes = (total % 12) as u8 + 1;
            competencias.push((Competencia { ano, mes }, parse_money("4000.00", "s").unwrap()));
        }
        competencias.push((Competencia { ano: 2019, mes: 11 }, parse_money("4000.00", "s").unwrap()));

        let entrada = RgpsEntrada {
            nome: None,
            especie: Some("42".into()),
            sexo: "feminino".into(),
            data_nascimento: Some(IsoDate::parse("1962-01-01", "n").unwrap()),
            data_referencia: Some(IsoDate::parse("2022-06-01", "dib").unwrap()),
            vinculos: vec![VinculoEntrada {
                inicio: IsoDate::parse("1984-01-01", "i").unwrap(),
                fim: Some(IsoDate::parse("2015-12-31", "f").unwrap()),
                tipo: "empregado".into(),
                empresa: None,
                fator: 1.0,
            }],
            competencias,
            opcoes: RgpsOpcoes {
                corrigir_salarios: true,
                aplicar_teto_vigente_der: true,
                aplicar_piso_automatico: true,
                ..RgpsOpcoes::default()
            },
        };
        let resultado = calcular(&entrada).unwrap();
        let transicoes = resultado.transicoes.as_ref().unwrap();
        let da = transicoes["regras"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "direito-adquirido")
            .unwrap();
        assert_eq!(da["elegivel"], serde_json::json!(true));
        let da_rmi: Decimal = da["rmi"].as_str().unwrap().parse().unwrap();
        // Com divisor 182 (em vez de 75) sobre salários ~4000 corrigidos até 11/2019 (~4500-5300),
        // o SB cai para ~2000-2300 (<= 2600), enquanto com divisor 75 seria > 4800.
        let obs = da["observacao"].as_str().unwrap();
        assert!(obs.contains("SB R$ "), "{obs}");
        assert!(da_rmi < Decimal::from(3500), "divisor 182 deve reduzir SB/RMI no direito adquirido: {da_rmi} ({obs})");
    }
}
