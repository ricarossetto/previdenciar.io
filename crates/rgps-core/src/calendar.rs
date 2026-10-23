use crate::money::RgpsError;

/// Competência AAAA-MM com ordenação léxica estável.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Competencia {
    pub ano: u16,
    pub mes: u8,
}

impl Competencia {
    pub fn parse(valor: &str, label: &str) -> Result<Self, RgpsError> {
        let bytes = valor.as_bytes();
        if valor.len() != 7 || bytes[4] != b'-' {
            return Err(RgpsError(format!("{label} inválida: {valor}. Use AAAA-MM.")));
        }
        let ano: u16 = valor[0..4]
            .parse()
            .map_err(|_| RgpsError(format!("{label} inválida: {valor}.")))?;
        let mes: u8 = valor[5..7]
            .parse()
            .map_err(|_| RgpsError(format!("{label} inválida: {valor}.")))?;
        if !(1..=12).contains(&mes) {
            return Err(RgpsError(format!(
                "{label} inválida: mês {mes} fora de 01..12."
            )));
        }
        Ok(Competencia { ano, mes })
    }

    pub fn to_string(&self) -> String {
        format!("{:04}-{:02}", self.ano, self.mes)
    }

    pub fn next(&self) -> Competencia {
        if self.mes == 12 {
            Competencia {
                ano: self.ano + 1,
                mes: 1,
            }
        } else {
            Competencia {
                ano: self.ano,
                mes: self.mes + 1,
            }
        }
    }

    /// Número de meses entre duas competências, incluindo ambas as pontas.
    pub fn meses_ate(&self, outra: &Competencia) -> i64 {
        let a = self.ano as i64 * 12 + self.mes as i64;
        let b = outra.ano as i64 * 12 + outra.mes as i64;
        b - a + 1
    }
}

impl std::fmt::Display for Competencia {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}", self.ano, self.mes)
    }
}

/// Data ISO AAAA-MM-DD com aritmética civil exata (dias entre datas, add dias).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IsoDate {
    pub ano: i32,
    pub mes: u8,
    pub dia: u8,
}

impl IsoDate {
    pub fn parse(valor: &str, label: &str) -> Result<Self, RgpsError> {
        let bytes = valor.as_bytes();
        if valor.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(RgpsError(format!(
                "{label} inválida: {valor}. Use AAAA-MM-DD."
            )));
        }
        let ano: i32 = valor[0..4]
            .parse()
            .map_err(|_| RgpsError(format!("{label} inválida: {valor}.")))?;
        let mes: u8 = valor[5..7]
            .parse()
            .map_err(|_| RgpsError(format!("{label} inválida: {valor}.")))?;
        let dia: u8 = valor[8..10]
            .parse()
            .map_err(|_| RgpsError(format!("{label} inválida: {valor}.")))?;
        if !(1..=12).contains(&mes) || dia == 0 || dia > dias_no_mes(ano, mes) {
            return Err(RgpsError(format!("{label} inválida: {valor}.")));
        }
        Ok(IsoDate { ano, mes, dia })
    }

    pub fn to_string(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.ano, self.mes, self.dia)
    }

    /// Dias entre `self` (início) e `outra` (fim); negativo se outra < self.
    pub fn days_between(&self, outra: &IsoDate) -> i64 {
        to_days_from_civil(outra.ano, outra.mes as i64, outra.dia as i64)
            - to_days_from_civil(self.ano, self.mes as i64, self.dia as i64)
    }

    pub fn add_days(&self, dias: i64) -> IsoDate {
        from_days_from_civil(to_days_from_civil(self.ano, self.mes as i64, self.dia as i64) + dias)
    }

    pub fn add_months_iso(&self, meses: i64) -> IsoDate {
        let total = self.ano as i64 * 12 + (self.mes as i64 - 1) + meses;
        let novo_ano = total.div_euclid(12) as i32;
        let novo_mes = total.rem_euclid(12) as u8 + 1;
        let ultimo = dias_no_mes(novo_ano, novo_mes);
        IsoDate {
            ano: novo_ano,
            mes: novo_mes,
            dia: self.dia.min(ultimo),
        }
    }
}

impl std::fmt::Display for IsoDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.ano, self.mes, self.dia)
    }
}

pub fn dias_no_mes(ano: i32, mes: u8) -> u8 {
    match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (ano % 4 == 0 && ano % 100 != 0) || ano % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Algoritmo civil (Howard Hinnant): dias desde 1970-01-01.
fn to_days_from_civil(ano: i32, mes: i64, dia: i64) -> i64 {
    let y = ano as i64 - if mes <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (mes + 9).rem_euclid(12);
    let doy = (153 * mp + 2) / 5 + dia - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn from_days_from_civil(z: i64) -> IsoDate {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let ano = (y + if m <= 2 { 1 } else { 0 }) as i32;
    IsoDate {
        ano,
        mes: m as u8,
        dia: d as u8,
    }
}

/// Idade em meses completos (mesma semântica de transicoes.mjs).
pub fn idade_em_meses(nascimento: &IsoDate, referencia: &IsoDate) -> i64 {
    let mut meses = (referencia.ano as i64 - nascimento.ano as i64) * 12
        + (referencia.mes as i64 - nascimento.mes as i64);
    if referencia.dia < nascimento.dia {
        meses -= 1;
    }
    meses.max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn competencia_ordena_e_soma_meses() {
        let a = Competencia::parse("1994-07", "t").unwrap();
        let b = Competencia::parse("1994-08", "t").unwrap();
        assert!(a < b);
        assert_eq!(a.meses_ate(&b), 2);
        assert_eq!(a.next().to_string(), "1994-08");
        assert_eq!(
            Competencia::parse("1994-12", "t").unwrap().next().to_string(),
            "1995-01"
        );
    }

    #[test]
    fn days_between_bissexto() {
        let a = IsoDate::parse("2024-02-28", "t").unwrap();
        let b = IsoDate::parse("2024-03-01", "t").unwrap();
        assert_eq!(a.days_between(&b), 2);
        assert_eq!(b.days_between(&a), -2);
        assert_eq!(a.add_days(2).to_string(), "2024-03-01");
    }

    #[test]
    fn idade_em_meses_completa() {
        let n = IsoDate::parse("1965-03-10", "t").unwrap();
        let r = IsoDate::parse("2026-10-09", "t").unwrap();
        assert_eq!(idade_em_meses(&n, &r), 61 * 12 + 6);
    }

    #[test]
    fn add_months_iso_clampa_dia() {
        let d = IsoDate::parse("2024-01-31", "t").unwrap();
        assert_eq!(d.add_months_iso(1).to_string(), "2024-02-29");
    }
}
