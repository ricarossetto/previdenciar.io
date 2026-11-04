use crate::atualizacao::teto_vigente;
use crate::calendar::{Competencia, IsoDate};
use crate::carencia::calcular_carencia;
use crate::especie::{self, Especie};
use crate::money::{add, div, money, mul, parse_money, Result, RgpsError};
use crate::rmi::{
    calcular_pbc_com_correcao, divisor_usado, salario_beneficio_sem_correcao,
};
use crate::tempo::{calcular_tempo, TempoTotal, VinculoEntrada};
use crate::transicoes::{build_transicoes, TransicoesEntrada};
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
                if *competencia <= COMPETENCIA_REFORMA {
                    t.marcos.ate_ec103 = (t.marcos.ate_ec103 - 30).max(0);
                } else {
                    t.marcos.apos_ec103 = (t.marcos.apos_ec103 - 30).max(0);
                }
            }
            let anos = t.total_dias_efetivos / 365;
            let meses = (t.total_dias_efetivos % 365) / 30;
            let dias = t.total_dias_efetivos - anos * 365 - meses * 30;
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
    let contribuicoes = quantidade.saturating_add(entrada.opcoes.meses_fora_do_tempo.len());
    carencia.competencias_salario = contribuicoes;
    carencia.competencias_vinculo = 0;
    let mut carencia_meses = contribuicoes
        .saturating_add(carencia.competencias_graca)
        .saturating_add(carencia.recolhimentos_atraso);

    // PBC / salário de benefício.
    let correcao_ativa = entrada.opcoes.corrigir_salarios;
    let correcao = if correcao_ativa {
        let referencia_correcao = entrada.opcoes.referencia_correcao.unwrap_or(ultima);
        match entrada
            .opcoes
            .perfil_correcao
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            // Correção pelo perfil oficial versionado: série por faixa de
            // vigência, com falha explícita quando a série oficial falta (ou
            // fallback documentado pelo INPC, se explicitamente habilitado).
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
                Some(match entrada.opcoes.descartar_menores_minimo {
                    Some(minimo) => crate::rmi::calcular_pbc_com_perfil_descarte_minimo(
                        &competencias,
                        &referencia_correcao,
                        minimo,
                        &catalogo,
                        &perfil,
                        politica_fallback,
                    )?,
                    None => crate::rmi::calcular_pbc_com_perfil(
                        &competencias,
                        &referencia_correcao,
                        entrada.opcoes.descarte,
                        &catalogo,
                        &perfil,
                        politica_fallback,
                    )?,
                })
            }
            None => Some(match entrada.opcoes.descartar_menores_minimo {
                Some(minimo) => crate::rmi::calcular_pbc_com_descarte_minimo(
                    &competencias,
                    &referencia_correcao,
                    minimo,
                )?,
                None => calcular_pbc_com_correcao(
                    &competencias,
                    &referencia_correcao,
                    entrada.opcoes.descarte,
                )?,
            }),
        }
    } else {
        None
    };

    let salario_beneficio = match &correcao {
        Some(pbc) => parse_money(&pbc.salario_beneficio, "salário de benefício")?,
        None => salario_beneficio_sem_correcao(&competencias, entrada.opcoes.divisor_minimo)?,
    };

    // EC 103, art. 26, §6º: o descarte dos menores remove as competências do
    // PBC e o tempo correspondente. O recálculo abaixo ajusta tempo, carência,
    // quantidade e, por consequência, coeficiente e transições.
    if let Some(pbc) = correcao.as_ref() {
        if entrada.opcoes.descartar_menores_minimo.is_some() && !pbc.parcelas_descartadas.is_empty() {
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
                        if *competencia <= COMPETENCIA_REFORMA {
                            t.marcos.ate_ec103 = (t.marcos.ate_ec103 - 30).max(0);
                        } else {
                            t.marcos.apos_ec103 = (t.marcos.apos_ec103 - 30).max(0);
                        }
                    }
                    let anos = t.total_dias_efetivos / 365;
                    let meses = (t.total_dias_efetivos % 365) / 30;
                    let dias = t.total_dias_efetivos - anos * 365 - meses * 30;
                    t.total = TempoTotal { anos, meses, dias };
                    t.carencia_meses = t.carencia_meses.saturating_sub(count);
                    t.avisos.push(format!(
                        "{count} competência(s) descartada(s) do PBC; o tempo foi reduzido em {count} mês(es) e o coeficiente/transições foram recalculados. Os vínculos de origem permanecem listados."
                    ));
                }
                // Carência conta contribuições; o descarte do PBC não a reduz.
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
    let base = anos_base(&entrada.sexo);
    let excedente = (anos - base).max(0);
    let coeficiente = if especie == Especie::TempoContribuicao {
        Decimal::from(60) + Decimal::from(2) * Decimal::from(excedente)
    } else {
        avaliacao.coeficiente
    };

    let mut rmi_bruta = mul(
        salario_beneficio,
        div(coeficiente, Decimal::from(100))?,
    );
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

    let teto_derivado = if entrada.opcoes.aplicar_teto_vigente_der {
        Some(teto_vigente(
            &entrada
                .opcoes
                .referencia_correcao
                .unwrap_or(ultima),
        )?)
    } else {
        entrada.opcoes.teto
    };
    if let Some(teto) = teto_derivado {
        if rmi_bruta > teto {
            rmi_bruta = teto;
            limite_aplicado = Some("teto");
        }
    }
    if let Some(piso) = entrada.opcoes.piso {
        if limite_aplicado.is_none() && rmi_bruta < piso {
            rmi_bruta = piso;
            limite_aplicado = Some("piso");
        }
    }

    // A RMI exibida é a da regra de transição vencedora; sem regra, fica a RMI
    // da espécie. A memória preserva a RMI bruta/base para auditoria.
    let mut rmi_exibicao = rmi_bruta;
    let mut coeficiente_exibicao = coeficiente;
    let mut regra_exibicao: Option<String> = None;
    let mut regra_nome_exibicao: Option<String> = None;
    let mut fator_exibicao: Option<Decimal> = None;

    // Transições EC 103 (quando há nascimento).
    let mut avisos = Vec::new();
    let mut transicoes = None;
    if let Some(nascimento) = entrada.data_nascimento.filter(|_| especie == Especie::TempoContribuicao) {
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
            salario_beneficio,
            teto: teto_derivado,
            piso: entrada.opcoes.piso,
            expectativa_sobrevida: entrada.opcoes.expectativa_sobrevida,
        })?;
        if let (Some(id), Some(rmi)) = (resultado.regra_escolhida, resultado.rmi_escolhida.as_ref()) {
            if let Ok(valor) = parse_money(rmi, "RMI da regra escolhida") {
                rmi_exibicao = valor;
                regra_exibicao = Some(id.to_string());
                if let Some(regra) = resultado.regras.iter().find(|r| r.id == id) {
                    regra_nome_exibicao = Some(regra.nome.clone());
                    if matches!(regra.criterio_rmi, "fator-previdenciario" | "100%-media") {
                        coeficiente_exibicao = Decimal::from(100);
                    } else if regra.criterio_rmi == "coeficiente-progressivo" {
                        if let Some(rmi_regra) = regra
                            .rmi
                            .as_deref()
                            .and_then(|value| parse_money(value, "RMI da regra progressiva").ok())
                        {
                            if salario_beneficio > Decimal::ZERO {
                                coeficiente_exibicao = div(
                                    mul(rmi_regra, Decimal::from(100)),
                                    salario_beneficio,
                                )?;
                            }
                        }
                    }
                    if regra.criterio_rmi == "fator-previdenciario" && salario_beneficio > Decimal::ZERO {
                        fator_exibicao = Some(div(rmi_exibicao, salario_beneficio)?);
                    }
                }
            }
        }
        transicoes = Some(serde_json::to_value(&resultado).expect("serialização de transições"));
    } else {
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
            format!("Regra de transição escolhida: {nome}")
        } else {
            match especie {
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
            descricao: "Fator previdenciário aplicado pela regra vencedora (RMI ÷ salário de benefício)".into(),
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
        if let Some(minimo) = entrada.opcoes.descartar_menores_minimo {
            let descartadas = correcao.as_ref().map(|pbc| pbc.descartadas).unwrap_or(0);
            avisos.push(format!(
                "Descarte mínimo da EC 103/2019, art. 26, § 6º, aplicado: {descartadas} menor(es) remuneração(ões) excluída(s) do PBC, mantendo {minimo} competência(s); tempo e carência foram recalculados."
            ));
        } else if entrada.opcoes.descarte {
            avisos.push("Descarte fixo dos 20% menores salários corrigidos selecionado; o descarte do art. 26, § 6º, da EC 103 está bloqueado.".into());
        } else {
            avisos.push("Nenhum descarte de salários selecionado; o descarte do art. 26, § 6º, da EC 103 está bloqueado.".into());
        }
        avisos.push(
            "Competências anteriores a 2001 podem divergir da tabela de transição oficial do INSS; o descarte dos 20% menores costuma absorver a diferença.".into(),
        );
    }
    if especie == Especie::TempoContribuicao && anos < base {
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
            regra_escolhida: if especie == Especie::TempoContribuicao {
                transicoes
                    .as_ref()
                    .and_then(|t| t.get("regra_escolhida"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            } else {
                avaliacao.regra_escolhida.map(str::to_string)
            },
            regra_nome: if especie == Especie::TempoContribuicao {
                regra_nome_exibicao.clone()
            } else {
                avaliacao
                    .regras
                    .iter()
                    .find(|r| Some(r.id) == avaliacao.regra_escolhida)
                    .map(|r| r.nome.clone())
            },
            elegivel: if especie == Especie::TempoContribuicao {
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
            divisor,
            divisor_minimo_aplicado: entrada.opcoes.divisor_minimo
                && (quantidade as u32) < crate::DIVISOR_MINIMO,
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
            piso: entrada.opcoes.piso.map(money),
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
        avisos,
    })
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
}
