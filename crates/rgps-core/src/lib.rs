pub mod atualizacao;
pub mod calendar;
pub mod carencia;
pub mod dados;
pub mod engine;
pub mod fiscal;
pub mod money;
pub mod projecao;
pub mod rmi;
pub mod tempo;
pub mod transicoes;

pub use engine::{calcular, RgpsEntrada, RgpsOpcoes, RgpsResultado};
pub use money::RgpsError;

/// Versão do motor (contrato do sidecar em GET /health).
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const SEXOS: [&str; 2] = ["feminino", "masculino"];
pub const PRIMEIRA_COMPETENCIA: &str = "1994-07";
pub const DIVISOR_MINIMO: u32 = 108;
pub const REFORMA_EC103: &str = "2019-11-13";
