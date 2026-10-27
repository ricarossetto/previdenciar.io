use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CalculationInput {
    pub segurado: InsuredInput,
    #[serde(rename = "dataReferencia")]
    pub data_referencia: Option<String>,
    pub vinculos: Vec<LinkInput>,
    pub competencias: Vec<WageInput>,
    pub opcoes: OptionsInput,
    pub revisao: ReviewInput,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InsuredInput {
    pub sexo: Option<String>,
    #[serde(rename = "dataNascimento")]
    pub data_nascimento: Option<String>,
    pub deficiencia: Option<String>,
    /// Espécie INSS declarada (ex.: "42"). Ausente em bases antigas.
    pub especie: Option<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LinkInput {
    pub inicio: Option<String>,
    pub fim: Option<String>,
    pub tipo: Option<String>,
    pub regime: Option<String>,
    pub confirmado: bool,
    pub evidencia: Option<String>,
    pub indicadores: Vec<String>,
    pub origem: Option<String>,
    pub prpps: bool,
    pub ctc_confirmada: bool,
    pub ctc_evidencia: Option<String>,
    pub fator: Option<f64>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WageInput {
    pub competencia: Option<String>,
    /// Decimal dot string; no binary float conversion or guessing Brazilian locale.
    pub salario: Option<String>,
    pub moeda: Option<String>,
    pub confirmada: bool,
    pub indicadores: Vec<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReviewInput {
    pub confirmada: bool,
    pub evidencia: Option<String>,
    pub politica: Option<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct OptionsInput {
    pub corrigir_salarios: Option<bool>,
    pub descarte: Option<bool>,
    pub divisor_minimo: Option<bool>,
    /// EC 103/2019, art. 26, § 6º: descarta os menores salários mantendo este
    /// mínimo de competências. Ausente desliga o descarte por redutores.
    pub descartar_menores_minimo: Option<u32>,
    pub referencia_correcao: Option<String>,
    pub teto: Option<String>,
    pub piso: Option<String>,
    pub aplicar_teto_vigente_der: Option<bool>,
    pub expectativa_sobrevida: Option<String>,
    pub graca: Vec<GracePeriod>,
    pub recolhimentos_atraso: Vec<String>,
    pub regra_alvo: Option<String>,
    /// Origem declarada da incapacidade (31/32): comum, acidente,
    /// doenca_ocupacional, doenca_do_trabalho ou doenca_listada.
    pub incapacidade_origem: Option<String>,
    /// 46: grau/agente declarado — "15", "20" ou "25" anos.
    pub grau_especial: Option<String>,
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
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GracePeriod { pub inicio: Option<String>, pub fim: Option<String> }

/// Stored only in Calculation.options.rule_policy, not in free-form personal notes.
/// No imported record is automatically marked confirmed or rewritten.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CasePolicy {
    pub lc142: Option<crate::pcd::Lc142Policy>,
    pub version: Option<String>,
    pub review_evidence: Option<String>,
    pub manual_link_kind: Option<String>,
    pub manual_currency: Option<String>,
    pub native_options: OptionsInput,
    pub exclude_imported_source: bool,
    pub source_exclusion_evidence: Option<String>,
    /// Declared by the operator when the reviewed identity intentionally differs
    /// from the imported CNIS (e.g. birth date correction). It does not rewrite
    /// the source; it records that the divergence was confirmed.
    pub identity_confirmed: bool,
}

#[derive(Clone, Serialize)]
pub struct Blocker {
    pub code: String,
    pub field: String,
    pub message: String,
}
