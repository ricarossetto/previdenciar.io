//! Regras de espécie e requisitos do RGPS.
//!
//! Cada espécie tem fórmula própria. Este módulo NÃO transforma uma espécie em
//! outra para "fazer o cálculo passar": 46 (especial) e 57 (professor) exigem
//! PPP/enquadramento e comprovação de magistério, e permanecem bloqueadas.
//! A incapacidade (31/32) é declarada pelo operador; o motor calcula a renda,
//! não o mérito médico, o nexo ou a data de início da incapacidade.

use crate::calendar::{idade_em_meses, IsoDate};
use crate::money::{Result, RgpsError};
use crate::rmi::coeficiente_progressivo;
use rust_decimal::Decimal;
use serde::Serialize;

pub const REFORMA: &str = "2019-11-13";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Especie {
    TempoContribuicao,
    Idade,
    IncapacidadePermanente,
    IncapacidadeTemporaria,
    Especial,
    Professor,
}

impl Especie {
    /// Códigos INSS aceitos. Base antiga sem espécie continua sendo 42.
    pub fn parse(codigo: Option<&str>) -> Result<Self> {
        let codigo = codigo.map(str::trim).unwrap_or("");
        match codigo {
            "" | "42" => Ok(Self::TempoContribuicao),
            "41" => Ok(Self::Idade),
            "32" => Ok(Self::IncapacidadePermanente),
            "31" => Ok(Self::IncapacidadeTemporaria),
            "46" => Ok(Self::Especial),
            "57" => Ok(Self::Professor),
            outro => Err(RgpsError(format!(
                "Espécie {outro} não tem fórmula implementada neste motor. Espécies atendidas: 42 (tempo de contribuição), 41 (idade), 32 (incapacidade permanente), 31 (incapacidade temporária), 46 (especial) e 57 (professor)."
            ))),
        }
    }

    pub fn codigo(self) -> &'static str {
        match self {
            Self::TempoContribuicao => "42",
            Self::Idade => "41",
            Self::IncapacidadePermanente => "32",
            Self::IncapacidadeTemporaria => "31",
            Self::Especial => "46",
            Self::Professor => "57",
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            Self::TempoContribuicao => "Aposentadoria por tempo de contribuição",
            Self::Idade => "Aposentadoria por idade",
            Self::IncapacidadePermanente => "Aposentadoria por incapacidade permanente",
            Self::IncapacidadeTemporaria => "Auxílio por incapacidade temporária",
            Self::Especial => "Aposentadoria especial",
            Self::Professor => "Aposentadoria de professor",
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Requisito {
    pub id: &'static str,
    pub descricao: String,
    pub exigido: String,
    pub apurado: String,
    pub cumprido: bool,
    /// Meses que faltam quando o requisito é mensurável.
    pub falta_meses: Option<i64>,
    /// Falso quando o requisito é declarado (perícia, PPP) e o motor não o verifica.
    pub verificado_pelo_motor: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegraAvaliada {
    pub id: &'static str,
    pub nome: String,
    pub fundamento: &'static str,
    pub elegivel: bool,
    pub requisitos: Vec<Requisito>,
    /// Soma dos meses faltantes; zero quando elegível.
    pub faltas_meses: i64,
    pub coeficiente: Decimal,
    pub observacao: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BeneficioApurado {
    pub especie: &'static str,
    pub nome: &'static str,
    pub coeficiente: Decimal,
    pub regra_escolhida: Option<&'static str>,
    pub regras: Vec<RegraAvaliada>,
    pub base_legal: Vec<&'static str>,
    /// Espécie 31: a RMI não pode exceder a média dos últimos 12 salários.
    pub limite_media_12: bool,
    pub observacoes: Vec<String>,
}

pub struct AvaliacaoEntrada<'a> {
    pub especie: Especie,
    pub sexo: &'a str,
    pub data_nascimento: Option<IsoDate>,
    pub data_referencia: Option<IsoDate>,
    pub carencia_meses: i64,
    /// Tempo apurado por vínculos; cai para competências quando não há vínculo.
    pub tempo_meses: i64,
    pub competencias: i64,
    /// Primeira competência/primeiro vínculo, para a filiação pré-reforma.
    pub inicio_filiacao: Option<IsoDate>,
    pub rural: bool,
    /// comum | acidente | doenca_ocupacional | doenca_do_trabalho | doenca_listada
    pub incapacidade_origem: Option<&'a str>,
    pub media_ultimos_12: Option<Decimal>,
    /// 46: grau/agente declarado — "15", "20" ou "25" anos de exposição.
    pub especial_grau: Option<&'a str>,
    /// 46: tempo de efetiva exposição declarado, em meses.
    pub especial_meses: Option<i64>,
    /// 46: o tempo especial foi completado até 13/11/2019 (direito adquirido)?
    pub especial_antes_reforma: Option<bool>,
    /// 57: tempo de efetivo magistério declarado, em meses.
    pub magisterio_meses: Option<i64>,
    /// 57: magistério exclusivo na educação infantil/fundamental/médio.
    pub magisterio_exclusivo: Option<bool>,
    /// 57: o tempo de magistério foi completado até 13/11/2019?
    pub magisterio_antes_reforma: Option<bool>,
    /// Meses de contribuição até 13/11/2019, para pontos e pedágio.
    pub tempo_ate_reforma_meses: i64,
}

fn requisito(
    id: &'static str,
    descricao: impl Into<String>,
    exigido: impl Into<String>,
    apurado: impl Into<String>,
    cumprido: bool,
    falta_meses: Option<i64>,
    verificado: bool,
) -> Requisito {
    Requisito {
        id,
        descricao: descricao.into(),
        exigido: exigido.into(),
        apurado: apurado.into(),
        cumprido,
        falta_meses,
        verificado_pelo_motor: verificado,
    }
}

fn anos_meses(meses: i64) -> String {
    let meses = meses.max(0);
    format!("{}a {}m", meses / 12, meses % 12)
}

fn idade(e: &AvaliacaoEntrada<'_>) -> Option<i64> {
    match (e.data_nascimento.as_ref(), e.data_referencia.as_ref()) {
        (Some(nasc), Some(der)) => Some(idade_em_meses(nasc, der)),
        _ => None,
    }
}

fn filiado_antes_reforma(e: &AvaliacaoEntrada<'_>) -> Option<bool> {
    let reforma = IsoDate::parse(REFORMA, "reforma").ok()?;
    e.inicio_filiacao.map(|inicio| inicio < reforma)
}

fn meses_contribuicao(e: &AvaliacaoEntrada<'_>) -> u32 {
    let meses = if e.tempo_meses > 0 {
        e.tempo_meses
    } else {
        e.competencias
    };
    meses.clamp(0, i64::from(u32::MAX)) as u32
}

fn origem_normalizada(e: &AvaliacaoEntrada<'_>) -> &'static str {
    match e
        .incapacidade_origem
        .map(str::trim)
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("acidente") | Some("acidente_de_trabalho") | Some("acidente do trabalho") => {
            "acidente"
        }
        Some("doenca_ocupacional") | Some("doença ocupacional") | Some("doenca_profissional") => {
            "doenca_ocupacional"
        }
        Some("doenca_do_trabalho") | Some("doença do trabalho") => "doenca_do_trabalho",
        Some("doenca_listada") | Some("doença listada") | Some("doenca_especifica") => {
            "doenca_listada"
        }
        _ => "comum",
    }
}

fn isento_carencia(origem: &str) -> bool {
    matches!(
        origem,
        "acidente" | "doenca_ocupacional" | "doenca_do_trabalho" | "doenca_listada"
    )
}

fn coeficiente_100(origem: &str) -> bool {
    matches!(origem, "acidente" | "doenca_ocupacional" | "doenca_do_trabalho")
}

fn idade_art18_meses(sexo: &str, ano: i32) -> i64 {
    if sexo == "feminino" {
        (60 * 12 + (ano - 2020).max(0) as i64 * 6).min(62 * 12)
    } else {
        65 * 12
    }
}

fn idade_minima_permanente(sexo: &str) -> i64 {
    if sexo == "feminino" {
        62 * 12
    } else {
        65 * 12
    }
}
#[allow(clippy::too_many_arguments)]
fn regra_idade(
    id: &'static str,
    nome: impl Into<String>,
    fundamento: &'static str,
    idade_min: i64,
    idade_apurada: Option<i64>,
    exigido_idade: String,
    requisito_carencia: Requisito,
    coeficiente: Decimal,
    observacao: Option<String>,
) -> RegraAvaliada {
    let (cumprido_idade, falta_idade) = match idade_apurada {
        Some(apurada) => (apurada >= idade_min, (idade_min - apurada).max(0)),
        None => (false, 0),
    };
    let requisitos = vec![
        requisito(
            "idade",
            "Idade mínima",
            exigido_idade,
            idade_apurada
                .map(anos_meses)
                .unwrap_or_else(|| "não informada".into()),
            cumprido_idade,
            if cumprido_idade { None } else { Some(falta_idade) },
            true,
        ),
        requisito_carencia,
    ];
    let elegivel = requisitos.iter().all(|r| r.cumprido);
    RegraAvaliada {
        id,
        nome: nome.into(),
        fundamento,
        elegivel,
        faltas_meses: requisitos.iter().filter_map(|r| r.falta_meses).sum(),
        requisitos,
        coeficiente,
        observacao,
    }
}

pub fn avaliar(e: &AvaliacaoEntrada<'_>) -> Result<BeneficioApurado> {
    match e.especie {
        Especie::TempoContribuicao => Ok(BeneficioApurado {
            especie: e.especie.codigo(),
            nome: e.especie.nome(),
            coeficiente: Decimal::ZERO,
            regra_escolhida: None,
            regras: vec![],
            base_legal: vec!["EC 103/2019, arts. 15 e 16 (transições)"],
            limite_media_12: false,
            observacoes: vec![
                "A espécie 42 é comparada pelas regras de transição da EC 103; este módulo não substitui aquelas regras."
                    .into(),
            ],
        }),
        Especie::Idade => avaliar_idade(e),
        Especie::IncapacidadePermanente => avaliar_incapacidade_permanente(e),
        Especie::IncapacidadeTemporaria => avaliar_incapacidade_temporaria(e),
        Especie::Especial => avaliar_especial(e),
        Especie::Professor => avaliar_professor(e),
    }
}
fn avaliar_idade(e: &AvaliacaoEntrada<'_>) -> Result<BeneficioApurado> {
    let apurada = idade(e).ok_or_else(|| {
        RgpsError(
            "Espécie 41 exige data de nascimento e data de referência para conferir a idade.".into(),
        )
    })?;
    let der = e
        .data_referencia
        .ok_or_else(|| RgpsError("Espécie 41 exige data de referência (DER).".into()))?;
    let meses = meses_contribuicao(e);
    let coeficiente = coeficiente_progressivo(e.sexo, meses);
    let base = if e.sexo == "feminino" { 15 } else { 20 };
    let mut regras = Vec::new();
    let mut observacoes = vec![format!(
        "Coeficiente 60% + 2% por ano acima de {base} anos, calculado sobre {} de contribuição (EC 103/2019, art. 26, §§ 2º e 5º).",
        anos_meses(i64::from(meses))
    )];

    if der.to_string().as_str() < REFORMA {
        // Regra vigente na DER anterior à reforma: 70% + 1% por grupo de 12
        // contribuições, limitado a 100% (Lei 8.213/1991, art. 50).
        let grupos = i64::from(meses) / 12;
        let coeficiente_pre = (Decimal::from(70) + Decimal::from(grupos)).min(Decimal::from(100));
        let idade_min = if e.sexo == "feminino" { 60 * 12 } else { 65 * 12 };
        let requisito_carencia = requisito(
            "carencia",
            "Carência",
            "180 contribuições",
            format!("{} meses", e.carencia_meses),
            e.carencia_meses >= 180,
            Some((180 - e.carencia_meses).max(0)),
            true,
        );
        regras.push(regra_idade(
            "pre-reforma",
            "Regra vigente antes da EC 103/2019",
            "Lei 8.213/1991, arts. 48 e 50",
            idade_min,
            Some(apurada),
            format!("{} anos", idade_min / 12),
            requisito_carencia,
            coeficiente_pre,
            Some(
                "RMI pré-reforma: 70% + 1% por grupo de 12 contribuições, até 100%. O PBC deve refletir a regra então vigente (80% maiores, se aplicável)."
                    .into(),
            ),
        ));
    } else {
        let idade_min = idade_minima_permanente(e.sexo);
        let requisito_carencia = requisito(
            "carencia",
            "Carência",
            "180 contribuições",
            format!("{} meses", e.carencia_meses),
            e.carencia_meses >= 180,
            Some((180 - e.carencia_meses).max(0)),
            true,
        );
        regras.push(regra_idade(
            "idade-permanente",
            "Regra permanente por idade",
            "CF art. 201, § 7º, I; EC 103/2019, art. 26, §§ 2º e 5º",
            idade_min,
            Some(apurada),
            format!("{} anos", idade_min / 12),
            requisito_carencia,
            coeficiente,
            None,
        ));
        if let Some(antes) = filiado_antes_reforma(e) {
            if antes {
                let idade_min = idade_art18_meses(e.sexo, der.ano);
                let requisito_tempo = requisito(
                    "tempo_contribuicao",
                    "Tempo de contribuição",
                    "180 meses",
                    format!("{} meses", e.tempo_meses.max(e.competencias)),
                    e.tempo_meses.max(e.competencias) >= 180,
                    Some((180 - e.tempo_meses.max(e.competencias)).max(0)),
                    true,
                );
                regras.push(regra_idade(
                    "idade-transicao",
                    "Regra de transição por idade (filiado até 13/11/2019)",
                    "EC 103/2019, art. 18; art. 26, §§ 2º, I, e 5º",
                    idade_min,
                    Some(apurada),
                    format!(
                        "{} anos em {} (60 anos em 2020, +6 meses por ano até 62)",
                        idade_min / 12,
                        der.ano
                    ),
                    requisito_tempo,
                    coeficiente,
                    Some(
                        "A idade da mulher sobe 6 meses por ano desde 2020 até 62 anos; o requisito é tempo de contribuição, não apenas carência."
                            .into(),
                    ),
                ));
            }
        } else {
            observacoes.push(
                "Sem a primeira filiação/competência não é possível confirmar a regra de transição do art. 18; somente a regra permanente foi avaliada."
                    .into(),
            );
        }

        if e.rural {
            let requisito_carencia = requisito(
                "carencia",
                "Carência rural",
                "180 meses de atividade rural",
                format!("{} meses", e.carencia_meses),
                e.carencia_meses >= 180,
                Some((180 - e.carencia_meses).max(0)),
                true,
            );
            regras.push(regra_idade(
                "idade-rural",
                "Regra rural (segurado especial)",
                "CF art. 201, § 7º, II; EC 103/2019, art. 26, §§ 2º e 5º",
                if e.sexo == "feminino" { 55 * 12 } else { 60 * 12 },
                Some(apurada),
                if e.sexo == "feminino" {
                    "55 anos".into()
                } else {
                    "60 anos".into()
                },
                requisito_carencia,
                coeficiente,
                Some(
                    "A carência rural usa os meses informados; o motor não separa atividade rural de urbana nem confirma a condição de segurado especial."
                        .into(),
                ),
            ));
        }

    }
    let elegivel = regras.iter().any(|r| r.elegivel);
    let escolhida = if elegivel {
        regras
            .iter()
            .find(|r| r.elegivel && r.id == "idade-rural")
            .or_else(|| regras.iter().find(|r| r.elegivel && r.id == "idade-transicao"))
            .or_else(|| regras.iter().find(|r| r.elegivel))
    } else {
        None
    };
    let referencia = escolhida.or_else(|| regras.iter().min_by_key(|r| r.faltas_meses));
    Ok(BeneficioApurado {
        especie: e.especie.codigo(),
        nome: e.especie.nome(),
        coeficiente: referencia.map(|r| r.coeficiente).unwrap_or(coeficiente),
        regra_escolhida: escolhida.map(|r| r.id),
        regras,
        base_legal: vec![
            "CF art. 201, § 7º, I e II",
            "EC 103/2019, art. 18 e art. 26, §§ 2º, 3º e 5º",
            "Lei 8.213/1991, arts. 25, II, 48 e 50",
        ],
        limite_media_12: false,
        observacoes,
    })
}

fn avaliar_incapacidade_permanente(e: &AvaliacaoEntrada<'_>) -> Result<BeneficioApurado> {
    let origem = origem_normalizada(e);
    let isento = isento_carencia(origem);
    let exige = if isento { 0 } else { 12 };
    let carencia = requisito(
        "carencia",
        "Carência",
        if isento {
            "isento (acidente, doença ocupacional ou doença listada)".to_string()
        } else {
            "12 contribuições".into()
        },
        format!("{} meses", e.carencia_meses),
        e.carencia_meses >= exige,
        Some((exige - e.carencia_meses).max(0)),
        true,
    );
    let incapacidade = requisito(
        "incapacidade",
        "Incapacidade permanente",
        "comprovação por perícia/laudo",
        "declarada pelo operador; não verificada pelo motor",
        true,
        None,
        false,
    );
    let der = e.data_referencia.map(|d| d.to_string()).unwrap_or_default();
    let pre_reforma = !der.is_empty() && der.as_str() < REFORMA;
    let coeficiente = if pre_reforma || coeficiente_100(origem) {
        Decimal::from(100)
    } else {
        coeficiente_progressivo(e.sexo, meses_contribuicao(e))
    };
    let mut observacoes = vec![
        "O motor calcula a renda mensal; não avalia incapacidade, nexo, DII ou qualidade de segurado. Exige laudo/CAT e conferência profissional."
            .into(),
    ];
    if origem == "comum" {
        observacoes.push(
            "Origem não declarada como acidente/doença ocupacional: aplicada a regra comum de 60% + 2%. Se houver CAT ou nexo ocupacional, refaça com a origem correta."
                .into(),
        );
    }
    if coeficiente_100(origem) {
        observacoes.push(
            "Origem acidentária/ocupacional declarada: 100% da média (EC 103/2019, art. 26, § 3º, II)."
                .into(),
        );
    }
    if pre_reforma {
        observacoes.push(
            "DER anterior à EC 103/2019: aplicado o art. 44 da Lei 8.213/1991 (100% do salário de benefício). A data de início da incapacidade (DII) precisa ser conferida; o STF (Tema 1300) distingue a DII anterior da posterior à reforma."
                .into(),
        );
    }
    let elegivel = carencia.cumprido;
    let regra = RegraAvaliada {
        id: "incapacidade-permanente",
        nome: "Aposentadoria por incapacidade permanente".into(),
        fundamento: if pre_reforma {
            "Lei 8.213/1991, art. 44"
        } else if coeficiente_100(origem) {
            "EC 103/2019, art. 26, §§ 2º, III, e 3º, II"
        } else {
            "EC 103/2019, art. 26, §§ 2º, III, e 5º"
        },
        elegivel,
        faltas_meses: carencia.falta_meses.unwrap_or(0),
        requisitos: vec![carencia, incapacidade],
        coeficiente,
        observacao: None,
    };
    Ok(BeneficioApurado {
        especie: e.especie.codigo(),
        nome: e.especie.nome(),
        coeficiente,
        regra_escolhida: if elegivel {
            Some("incapacidade-permanente")
        } else {
            None
        },
        regras: vec![regra],
        base_legal: vec![
            "Lei 8.213/1991, arts. 25, I, 26, II, e 44",
            "EC 103/2019, art. 26, §§ 2º, III, 3º, II, e 5º",
        ],
        limite_media_12: false,
        observacoes,
    })
}

fn avaliar_incapacidade_temporaria(e: &AvaliacaoEntrada<'_>) -> Result<BeneficioApurado> {
    let origem = origem_normalizada(e);
    let isento = isento_carencia(origem);
    let exige = if isento { 0 } else { 12 };
    let carencia = requisito(
        "carencia",
        "Carência",
        if isento {
            "isento (acidente, doença ocupacional ou doença listada)".to_string()
        } else {
            "12 contribuições".into()
        },
        format!("{} meses", e.carencia_meses),
        e.carencia_meses >= exige,
        Some((exige - e.carencia_meses).max(0)),
        true,
    );
    let incapacidade = requisito(
        "incapacidade",
        "Incapacidade temporária",
        "comprovação por perícia/laudo",
        "declarada pelo operador; não verificada pelo motor",
        true,
        None,
        false,
    );
    let elegivel = carencia.cumprido;
    let mut observacoes = vec![
        "Renda mensal de 91% do salário de benefício (Lei 8.213/1991, art. 61), limitada à média dos últimos 12 salários de contribuição (art. 29, § 10)."
            .into(),
        "O motor não avalia a duração do afastamento, a perícia ou a data de início da incapacidade."
            .into(),
    ];
    if origem == "comum" {
        observacoes.push(
            "Origem não declarada como acidente/doença ocupacional; a carência de 12 meses foi exigida. Se houver CAT/doença listada, refaça com a origem correta."
                .into(),
        );
    }
    let regra = RegraAvaliada {
        id: "incapacidade-temporaria",
        nome: "Auxílio por incapacidade temporária".into(),
        fundamento: "Lei 8.213/1991, arts. 25, I, 26, II, 29, § 10, e 61",
        elegivel,
        faltas_meses: carencia.falta_meses.unwrap_or(0),
        requisitos: vec![carencia, incapacidade],
        coeficiente: Decimal::from(91),
        observacao: None,
    };
    Ok(BeneficioApurado {
        especie: e.especie.codigo(),
        nome: e.especie.nome(),
        coeficiente: Decimal::from(91),
        regra_escolhida: if elegivel {
            Some("incapacidade-temporaria")
        } else {
            None
        },
        regras: vec![regra],
        base_legal: vec!["Lei 8.213/1991, arts. 25, I, 26, II, 29, § 10, e 61"],
        limite_media_12: true,
        observacoes,
    })
}

fn declarado(
    id: &'static str,
    descricao: &str,
    exigido: impl Into<String>,
    apurado: String,
    cumprido: bool,
    falta_meses: Option<i64>,
) -> Requisito {
    requisito(id, descricao, exigido, apurado, cumprido, falta_meses, false)
}

fn compor_regra(
    id: &'static str,
    nome: &str,
    fundamento: &'static str,
    coeficiente: Decimal,
    requisitos: Vec<Requisito>,
    observacao: Option<String>,
) -> RegraAvaliada {
    let elegivel = requisitos.iter().all(|r| r.cumprido);
    let faltas_meses = requisitos.iter().filter_map(|r| r.falta_meses).sum();
    RegraAvaliada {
        id,
        nome: nome.into(),
        fundamento,
        elegivel,
        requisitos,
        faltas_meses,
        coeficiente,
        observacao,
    }
}

fn com_requisitos(mut regra: RegraAvaliada, extras: Vec<Requisito>) -> RegraAvaliada {
    regra.requisitos.extend(extras);
    regra.elegivel = regra.requisitos.iter().all(|r| r.cumprido);
    regra.faltas_meses = regra.requisitos.iter().filter_map(|r| r.falta_meses).sum();
    regra
}

fn avaliar_especial(e: &AvaliacaoEntrada<'_>) -> Result<BeneficioApurado> {
    let grau = e.especial_grau.unwrap_or("").trim();
    let (min_anos, idade_min) = match grau {
        "15" => (15_i64, 55_i64),
        "20" => (20, 58),
        "25" => (25, 60),
        _ => (0, 0),
    };
    let tempo = e.especial_meses.unwrap_or(0).max(0);
    let min_meses = min_anos * 12;
    let tempo_ok = min_anos > 0 && tempo >= min_meses;
    let falta_tempo = (min_meses - tempo).max(0);
    let carencia = e.carencia_meses.max(0);
    let carencia_ok = carencia >= 180;
    let idade_meses = idade(e).unwrap_or(0);
    // Art. 21 da EC 103/2019: pontos = idade + tempo TOTAL de contribuicao.
    // O tempo de efetiva exposicao e um requisito separado, nao uma parcela
    // adicional da soma. Comparar meses preserva fracoes de ano.
    let pontos_meses = idade_meses + e.tempo_meses.max(0);
    let pontos_min = match min_anos {
        15 => 66,
        20 => 76,
        25 => 86,
        _ => 86,
    };
    let inicio_antes = filiado_antes_reforma(e).unwrap_or(false);
    let grau_req = declarado(
        "grau_especial",
        "Grau/agente declarado (15, 20 ou 25 anos)",
        "15, 20 ou 25 anos",
        if min_anos > 0 {
            format!("{min_anos} anos")
        } else {
            "não informado".into()
        },
        min_anos > 0,
        None,
    );
    let especial_req = declarado(
        "tempo_especial",
        "Tempo de efetiva exposição declarado",
        format!("{min_anos} anos"),
        anos_meses(tempo),
        tempo_ok,
        if tempo_ok { None } else { Some(falta_tempo) },
    );
    let carencia_req = requisito(
        "carencia",
        "Carência",
        "180 contribuições",
        format!("{carencia} meses"),
        carencia_ok,
        if carencia_ok {
            None
        } else {
            Some((180 - carencia).max(0))
        },
        true,
    );

    let antes_req = declarado(
        "especial_antes_reforma",
        "Tempo especial completo até 13/11/2019",
        "sim",
        if e.especial_antes_reforma == Some(true) {
            "sim".into()
        } else {
            "não informado".into()
        },
        e.especial_antes_reforma == Some(true),
        None,
    );
    let coeficiente = coeficiente_progressivo(e.sexo, e.tempo_meses.max(0) as u32);
    let direito = compor_regra(
        "especial-direito-adquirido",
        "Direito adquirido — exposição completa até 13/11/2019",
        "Lei 8.213/1991, art. 57, § 1º; EC 103/2019",
        Decimal::from(100),
        vec![
            grau_req.clone(),
            especial_req.clone(),
            carencia_req.clone(),
            antes_req,
        ],
        Some("RMI de 100% do salário de benefício; PPP e enquadramento são declarados, não verificados pelo motor.".into()),
    );
    let permanente_base = regra_idade(
        "especial-permanente",
        "Regra permanente — idade + tempo especial",
        "EC 103/2019, arts. 19 e 26, §§ 2º, IV, e 5º",
        idade_min * 12,
        idade(e),
        format!("{idade_min} anos"),
        carencia_req.clone(),
        coeficiente,
        None,
    );
    let permanente = com_requisitos(permanente_base, vec![grau_req.clone(), especial_req.clone()]);
    let pontos_req = requisito(
        "pontos_especial",
        "Pontos (idade + tempo de contribuição)",
        format!("{pontos_min} pontos"),
        format!("{} pontos e {} meses na soma", pontos_meses / 12, pontos_meses % 12),
        pontos_meses >= pontos_min * 12 && inicio_antes,
        if pontos_meses >= pontos_min * 12 && inicio_antes {
            None
        } else {
            Some((pontos_min * 12 - pontos_meses).max(0))
        },
        false,
    );
    let transicao = compor_regra(
        "especial-transicao",
        "Transição por pontos",
        "EC 103/2019, arts. 21 e 26, §§ 2º, IV, e 5º",
        coeficiente,
        vec![grau_req, especial_req, carencia_req, pontos_req],
        Some("Só para quem começou a exposição antes de 13/11/2019; a soma é declarada.".into()),
    );
    let regras = vec![direito, permanente, transicao];
    let escolhida = regras
        .iter()
        .filter(|r| r.elegivel)
        .max_by_key(|r| r.coeficiente);
    let coeficiente_escolhido = escolhida.map(|r| r.coeficiente).unwrap_or(coeficiente);
    let regra_escolhida = escolhida.map(|r| r.id);
    let mut observacoes = vec![
        "A exposição especial é declarada; o PPP, o enquadramento e a efetiva exposição não são verificados pelo motor. O CNIS não basta.".to_string(),
        "O direito adquirido usa 100% da média; a regra permanente e a transição usam 60% + 2% por ano acima de 20 (homem) ou 15 (mulher).".to_string(),
    ];
    if min_anos == 0 {
        observacoes.push(
            "Informe o grau/agente (15, 20 ou 25 anos) e o tempo especial para o motor avaliar."
                .into(),
        );
    }
    Ok(BeneficioApurado {
        especie: e.especie.codigo(),
        nome: e.especie.nome(),
        coeficiente: coeficiente_escolhido,
        regra_escolhida,
        regras,
        base_legal: vec![
            "Lei 8.213/1991, arts. 25, II, 57 e 58",
            "EC 103/2019, arts. 19, 21 e 26",
        ],
        limite_media_12: false,
        observacoes,
    })
}

fn avaliar_professor(e: &AvaliacaoEntrada<'_>) -> Result<BeneficioApurado> {
    let min_anos = if e.sexo == "feminino" { 25_i64 } else { 30_i64 };
    let min_meses = min_anos * 12;
    let mag = e.magisterio_meses.unwrap_or(0).max(0);
    let mag_ok = mag >= min_meses;
    let falta_mag = (min_meses - mag).max(0);
    let carencia = e.carencia_meses.max(0);
    let carencia_ok = carencia >= 180;
    let idade_meses = idade(e).unwrap_or(0);
    let idade_anos = idade_meses / 12;
    // Art. 15, § 3º: desde 2020 soma-se 1 ponto por ano, com teto 92/100.
    // O requisito e o somatorio de idade e contribuicao, sem duplicar magisterio.
    let pontos_meses = idade_meses + e.tempo_meses.max(0);
    let ano_der = e.data_referencia.map(|date| date.ano).unwrap_or(2019);
    let incremento = i64::from((ano_der - 2019).max(0));
    let pontos_min = if e.sexo == "feminino" {
        (81_i64 + incremento).min(92)
    } else {
        (91_i64 + incremento).min(100)
    };
    let inicio_antes = filiado_antes_reforma(e).unwrap_or(false);
    // Art. 19, § 1º, II: RGPS, 57/60 anos e 25 anos de magisterio.
    let idade_min = if e.sexo == "feminino" { 57_i64 } else { 60_i64 };
    let idade_ped = if e.sexo == "feminino" { 52_i64 } else { 55_i64 };
    let mag_req = declarado(
        "tempo_magisterio",
        "Tempo de efetivo magistério declarado",
        format!("{min_anos} anos"),
        anos_meses(mag),
        mag_ok,
        if mag_ok { None } else { Some(falta_mag) },
    );
    let exclusivo_req = declarado(
        "magisterio_exclusivo",
        "Magistério exclusivo na educação infantil, fundamental ou médio",
        "sim",
        if e.magisterio_exclusivo == Some(true) {
            "sim".into()
        } else {
            "não informado".into()
        },
        e.magisterio_exclusivo == Some(true),
        None,
    );
    let carencia_req = requisito(
        "carencia",
        "Carência",
        "180 contribuições",
        format!("{carencia} meses"),
        carencia_ok,
        if carencia_ok {
            None
        } else {
            Some((180 - carencia).max(0))
        },
        true,
    );
    let antes_req = declarado(
        "magisterio_antes_reforma",
        "Tempo de magistério completo até 13/11/2019",
        "sim",
        if e.magisterio_antes_reforma == Some(true) {
            "sim".into()
        } else {
            "não informado".into()
        },
        e.magisterio_antes_reforma == Some(true),
        None,
    );

    let coeficiente = coeficiente_progressivo(e.sexo, e.tempo_meses.max(0) as u32);
    let direito = compor_regra(
        "professor-direito-adquirido",
        "Direito adquirido — magistério completo até 13/11/2019",
        "Lei 8.213/1991, art. 56; EC 103/2019",
        Decimal::ZERO,
        vec![
            mag_req.clone(),
            exclusivo_req.clone(),
            carencia_req.clone(),
            antes_req,
        ],
        Some("A RMI antiga de 80% × fator não é calculada neste motor; nenhum valor é inventado.".into()),
    );
    // A regra permanente exige 25 anos para ambos os sexos; nas transicoes
    // e no direito adquirido conservam-se 25 anos (mulher) / 30 (homem).
    let mag_permanente_req = declarado(
        "tempo_magisterio_permanente",
        "Tempo de efetivo magistério — regra permanente",
        "25 anos",
        anos_meses(mag),
        mag >= 25 * 12,
        if mag >= 25 * 12 { None } else { Some(25 * 12 - mag) },
    );
    let permanente_base = regra_idade(
        "professor-permanente",
        "Regra permanente — idade + magistério",
        "EC 103/2019, art. 19, § 1º, II; art. 26, §§ 2º, I, e 5º",
        idade_min * 12,
        idade(e),
        format!("{idade_min} anos"),
        carencia_req.clone(),
        coeficiente,
        None,
    );
    let permanente = com_requisitos(
        permanente_base,
        vec![mag_permanente_req, exclusivo_req.clone()],
    );
    let pontos_req = requisito(
        "pontos_professor",
        "Pontos (idade + tempo de contribuição)",
        format!("{pontos_min} pontos"),
        format!("{} pontos e {} meses na soma", pontos_meses / 12, pontos_meses % 12),
        pontos_meses >= pontos_min * 12 && inicio_antes,
        if pontos_meses >= pontos_min * 12 && inicio_antes {
            None
        } else {
            Some((pontos_min * 12 - pontos_meses).max(0))
        },
        false,
    );
    let transicao = compor_regra(
        "professor-pontos",
        "Transição por pontos",
        "EC 103/2019, arts. 15 e 26, §§ 2º, I, e 5º",
        coeficiente,
        vec![
            mag_req.clone(),
            exclusivo_req.clone(),
            carencia_req.clone(),
            pontos_req,
        ],
        Some("Só para quem tinha filiação antes de 13/11/2019; a soma é declarada.".into()),
    );
    let mag_ate_reforma = mag.min(e.tempo_ate_reforma_meses.max(0));
    let pedagio_meses = (min_meses - mag_ate_reforma).max(0);
    let total_exigido = min_meses + pedagio_meses;
    let pedagio_req = requisito(
        "pedagio_professor",
        "Pedágio de 100% do tempo que faltava em 13/11/2019",
        format!("{total_exigido} meses de tempo total"),
        anos_meses(e.tempo_meses.max(0)),
        e.tempo_meses.max(0) >= total_exigido,
        if e.tempo_meses.max(0) >= total_exigido {
            None
        } else {
            Some(total_exigido - e.tempo_meses.max(0))
        },
        false,
    );
    let idade_req = requisito(
        "idade_pedagio",
        "Idade mínima no pedágio",
        format!("{idade_ped} anos"),
        idade(e)
            .map(anos_meses)
            .unwrap_or_else(|| "não informada".into()),
        idade_anos >= idade_ped,
        if idade_anos >= idade_ped {
            None
        } else {
            Some((idade_ped - idade_anos).max(0) * 12)
        },
        true,
    );
    let pedagogio = compor_regra(
        "professor-pedagio",
        "Transição por pedágio de 100%",
        "EC 103/2019, arts. 20 e 26, § 3º, I",
        Decimal::from(100),
        vec![
            mag_req,
            exclusivo_req,
            carencia_req,
            pedagio_req,
            idade_req,
        ],
        Some("RMI de 100% da média; o pedágio é o tempo que faltava em 13/11/2019.".into()),
    );
    let regras = vec![direito, permanente, transicao, pedagogio];
    let escolhida = regras
        .iter()
        .filter(|r| r.elegivel)
        .max_by_key(|r| r.coeficiente);
    let coeficiente_escolhido = escolhida.map(|r| r.coeficiente).unwrap_or(coeficiente);
    let regra_escolhida = escolhida.map(|r| r.id);
    let observacoes = vec![
        "O tempo de magistério e a exclusividade são declarados; a prova não é verificada pelo motor.".to_string(),
        "O direito adquirido do professor (80% × fator) não é calculado; nenhum valor é inventado.".to_string(),
        "A transição do pedágio usa 100% da média; a regra permanente e a de pontos usam 60% + 2%.".to_string(),
    ];
    Ok(BeneficioApurado {
        especie: e.especie.codigo(),
        nome: e.especie.nome(),
        coeficiente: coeficiente_escolhido,
        regra_escolhida,
        regras,
        base_legal: vec![
            "Lei 8.213/1991, arts. 25, II, 29 e 56",
            "EC 103/2019, arts. 15, 18, 20 e 26",
        ],
        limite_media_12: false,
        observacoes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(valor: &str) -> IsoDate {
        IsoDate::parse(valor, "teste").unwrap()
    }

    fn base(especie: Especie) -> AvaliacaoEntrada<'static> {
        AvaliacaoEntrada {
            especie,
            sexo: "masculino",
            data_nascimento: Some(data("1956-01-10")),
            data_referencia: Some(data("2026-01-10")),
            carencia_meses: 300,
            tempo_meses: 300,
            competencias: 300,
            inicio_filiacao: Some(data("1990-01-01")),
            rural: false,
            incapacidade_origem: Some("comum"),
            media_ultimos_12: Some(Decimal::from(3000)),
            especial_grau: None,
            especial_meses: None,
            especial_antes_reforma: None,
            magisterio_meses: None,
            magisterio_exclusivo: None,
            magisterio_antes_reforma: None,
            tempo_ate_reforma_meses: 240,
        }
    }

    #[test]
    fn idade_homem_70_anos_com_25_anos_coeficiente_70() {
        let avaliado = avaliar(&base(Especie::Idade)).unwrap();
        assert_eq!(avaliado.especie, "41");
        assert_eq!(avaliado.coeficiente, Decimal::from(70));
        assert!(avaliado.regra_escolhida.is_some());
        let permanente = avaliado
            .regras
            .iter()
            .find(|r| r.id == "idade-permanente")
            .unwrap();
        assert!(permanente.elegivel, "{permanente:?}");
    }

    #[test]
    fn idade_mulher_precisa_62_anos() {
        let mut e = base(Especie::Idade);
        e.sexo = "feminino";
        e.data_nascimento = Some(data("1964-06-01"));
        e.data_referencia = Some(data("2026-01-10"));
        e.tempo_meses = 300;
        e.carencia_meses = 300;
        let avaliado = avaliar(&e).unwrap();
        let permanente = avaliado
            .regras
            .iter()
            .find(|r| r.id == "idade-permanente")
            .unwrap();
        assert!(!permanente.elegivel);
        let idade = permanente.requisitos.iter().find(|r| r.id == "idade").unwrap();
        assert_eq!(idade.falta_meses, Some(5));
        // 25 anos: 60 + 2 x (25 - 15) = 80
        assert_eq!(avaliado.coeficiente, Decimal::from(80));
    }

    #[test]
    fn idade_transicao_art18_usa_60_anos_da_mulher_em_2020() {
        let mut e = base(Especie::Idade);
        e.sexo = "feminino";
        e.data_nascimento = Some(data("1960-01-10"));
        e.data_referencia = Some(data("2020-06-10"));
        e.tempo_meses = 240;
        e.carencia_meses = 240;
        e.inicio_filiacao = Some(data("1990-01-01"));
        let avaliado = avaliar(&e).unwrap();
        let transicao = avaliado
            .regras
            .iter()
            .find(|r| r.id == "idade-transicao")
            .expect("regra do art. 18 avaliada");
        assert!(transicao.elegivel, "{transicao:?}");
        assert_eq!(
            transicao.fundamento,
            "EC 103/2019, art. 18; art. 26, §§ 2º, I, e 5º"
        );
    }

    #[test]
    fn idade_pre_reforma_70_mais_1_por_ano() {
        let mut e = base(Especie::Idade);
        e.data_nascimento = Some(data("1950-01-10"));
        e.data_referencia = Some(data("2018-01-10"));
        let avaliado = avaliar(&e).unwrap();
        // 25 grupos de 12 = 70 + 25 = 95
        assert_eq!(avaliado.coeficiente, Decimal::from(95));
        assert_eq!(avaliado.regra_escolhida, Some("pre-reforma"));
    }

    #[test]
    fn incapacidade_permanente_comum_60_2() {
        let mut e = base(Especie::IncapacidadePermanente);
        e.carencia_meses = 12;
        e.tempo_meses = 300;
        let avaliado = avaliar(&e).unwrap();
        assert_eq!(avaliado.coeficiente, Decimal::from(70));
        assert!(avaliado.regra_escolhida.is_some());
    }

    #[test]
    fn incapacidade_permanente_acidentaria_100() {
        let mut e = base(Especie::IncapacidadePermanente);
        e.incapacidade_origem = Some("acidente");
        e.carencia_meses = 1;
        let avaliado = avaliar(&e).unwrap();
        assert_eq!(avaliado.coeficiente, Decimal::from(100));
        let carencia = avaliado.regras[0]
            .requisitos
            .iter()
            .find(|r| r.id == "carencia")
            .unwrap();
        assert!(carencia.cumprido);
    }

    #[test]
    fn incapacidade_temporaria_91_e_limite_dos_12() {
        let mut e = base(Especie::IncapacidadeTemporaria);
        e.carencia_meses = 5;
        let avaliado = avaliar(&e).unwrap();
        assert_eq!(avaliado.coeficiente, Decimal::from(91));
        assert!(avaliado.limite_media_12);
        assert!(!avaliado.regras[0].elegivel);
        assert_eq!(avaliado.regras[0].faltas_meses, 7);
    }

    #[test]
    fn especial_e_professor_sao_especies_proprias() {
        assert_eq!(Especie::parse(Some("46")).unwrap(), Especie::Especial);
        assert_eq!(Especie::parse(Some("57")).unwrap(), Especie::Professor);
        assert_eq!(
            Especie::parse(Some("42")).unwrap(),
            Especie::TempoContribuicao
        );
        assert_eq!(Especie::parse(None).unwrap(), Especie::TempoContribuicao);
    }

    #[test]
    fn especial_com_direito_adquirido_usa_100() {
        let mut e = base(Especie::Especial);
        e.especial_grau = Some("25");
        e.especial_meses = Some(300);
        e.especial_antes_reforma = Some(true);
        let avaliado = avaliar(&e).unwrap();
        assert_eq!(avaliado.coeficiente, Decimal::from(100));
        assert_eq!(
            avaliado.regra_escolhida,
            Some("especial-direito-adquirido")
        );
        let regra = avaliado
            .regras
            .iter()
            .find(|r| r.id == "especial-direito-adquirido")
            .unwrap();
        assert!(regra.elegivel, "{regra:?}");
        assert!(regra
            .requisitos
            .iter()
            .any(|r| r.id == "tempo_especial" && !r.verificado_pelo_motor));
    }

    #[test]
    fn especial_permanente_60_2() {
        let mut e = base(Especie::Especial);
        e.especial_grau = Some("25");
        e.especial_meses = Some(300);
        e.especial_antes_reforma = Some(false);
        e.data_nascimento = Some(data("1960-01-10"));
        e.data_referencia = Some(data("2020-06-10"));
        e.tempo_meses = 360;
        let avaliado = avaliar(&e).unwrap();
        assert_eq!(avaliado.coeficiente, Decimal::from(80));
        let permanente = avaliado
            .regras
            .iter()
            .find(|r| r.id == "especial-permanente")
            .unwrap();
        assert!(permanente.elegivel, "{permanente:?}");
        let especial = permanente
            .requisitos
            .iter()
            .find(|r| r.id == "tempo_especial")
            .unwrap();
        assert!(!especial.verificado_pelo_motor);
    }

    #[test]
    fn especial_pontos_nao_soma_exposicao_duas_vezes() {
        let mut e = base(Especie::Especial);
        e.especial_grau = Some("25");
        e.especial_meses = Some(300);
        e.especial_antes_reforma = Some(false);
        e.tempo_meses = 300;
        e.data_nascimento = Some(data("1971-01-10")); // 55 + 25 = 80, abaixo de 86.
        let avaliado = avaliar(&e).unwrap();
        let transicao = avaliado.regras.iter().find(|r| r.id == "especial-transicao").unwrap();
        let pontos = transicao.requisitos.iter().find(|r| r.id == "pontos_especial").unwrap();
        assert!(!pontos.cumprido);
        assert_eq!(pontos.falta_meses, Some(72));
        assert!(!transicao.elegivel);
        e.data_nascimento = Some(data("1965-02-10")); // 60 anos e 11 meses.
        e.tempo_meses = 301; // +25 anos e 1 mes = 86 pontos.
        let avaliado = avaliar(&e).unwrap();
        let transicao = avaliado.regras.iter().find(|r| r.id == "especial-transicao").unwrap();
        let pontos = transicao.requisitos.iter().find(|r| r.id == "pontos_especial").unwrap();
        assert!(pontos.cumprido);
        assert!(transicao.elegivel);
    }

    #[test]
    fn professor_permanente_exige_57_60_e_25_anos_de_magisterio() {
        let mut e = base(Especie::Professor);
        e.magisterio_meses = Some(300);
        e.magisterio_exclusivo = Some(true);
        e.tempo_meses = 300;
        e.inicio_filiacao = Some(data("2020-01-01"));
        e.data_nascimento = Some(data("1966-01-10")); // 60 anos.
        let avaliado = avaliar(&e).unwrap();
        let regra = avaliado.regras.iter().find(|r| r.id == "professor-permanente").unwrap();
        assert!(regra.elegivel, "{regra:?}");
        assert!(regra.requisitos.iter().any(|r| r.id == "tempo_magisterio_permanente" && r.cumprido));
        e.sexo = "feminino";
        e.data_nascimento = Some(data("1969-01-10")); // 57 anos.
        let avaliado = avaliar(&e).unwrap();
        let regra = avaliado.regras.iter().find(|r| r.id == "professor-permanente").unwrap();
        assert!(regra.elegivel, "{regra:?}");
    }

    #[test]
    fn professor_pontos_progridem_por_ano_ate_o_teto() {
        let mut e = base(Especie::Professor);
        e.sexo = "feminino";
        e.magisterio_meses = Some(300);
        e.magisterio_exclusivo = Some(true);
        e.tempo_meses = 300;
        e.data_nascimento = Some(data("1969-01-10")); // 57 + 25 = 82.
        let avaliado = avaliar(&e).unwrap();
        let regra = avaliado.regras.iter().find(|r| r.id == "professor-pontos").unwrap();
        let pontos = regra.requisitos.iter().find(|r| r.id == "pontos_professor").unwrap();
        assert_eq!(pontos.exigido, "88 pontos"); // 2026: 81 + 7.
        assert!(!pontos.cumprido);
        e.data_referencia = Some(data("2035-01-10"));
        let avaliado = avaliar(&e).unwrap();
        let regra = avaliado.regras.iter().find(|r| r.id == "professor-pontos").unwrap();
        let pontos = regra.requisitos.iter().find(|r| r.id == "pontos_professor").unwrap();
        assert_eq!(pontos.exigido, "92 pontos"); // teto feminino.
        e.sexo = "masculino";
        let avaliado = avaliar(&e).unwrap();
        let regra = avaliado.regras.iter().find(|r| r.id == "professor-pontos").unwrap();
        let pontos = regra.requisitos.iter().find(|r| r.id == "pontos_professor").unwrap();
        assert_eq!(pontos.exigido, "100 pontos"); // teto masculino.
    }

    #[test]
    fn professor_pedagio_100() {
        let mut e = base(Especie::Professor);
        e.magisterio_meses = Some(360);
        e.magisterio_exclusivo = Some(true);
        e.magisterio_antes_reforma = Some(false);
        e.data_nascimento = Some(data("1965-01-10"));
        e.data_referencia = Some(data("2020-06-10"));
        e.tempo_meses = 480;
        e.tempo_ate_reforma_meses = 240;
        let avaliado = avaliar(&e).unwrap();
        assert_eq!(avaliado.coeficiente, Decimal::from(100));
        assert_eq!(
            avaliado.regra_escolhida,
            Some("professor-pedagio")
        );
        let pedagio = avaliado
            .regras
            .iter()
            .find(|r| r.id == "professor-pedagio")
            .unwrap();
        assert!(pedagio.elegivel, "{pedagio:?}");
        assert!(pedagio
            .requisitos
            .iter()
            .any(|r| r.id == "magisterio_exclusivo" && !r.verificado_pelo_motor));
    }
}







