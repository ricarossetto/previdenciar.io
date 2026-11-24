// GERADO a partir de OpenPrev/openprev/data/reajustes_beneficios.csv.
// Índices oficiais de reajuste dos benefícios em manutenção (Portarias MPS/MF/MTP/ME/SEPRT, 2019-2026).
use crate::calendar::IsoDate;
use crate::money::to_cents;
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub struct ReajusteManutencao {
    pub ano: i32,
    pub mes_inicio: u8,
    pub percentual: &'static str,
    pub fonte: &'static str,
}

pub static REAJUSTES_BENEFICIOS: [ReajusteManutencao; 96] = [
    ReajusteManutencao { ano: 2019, mes_inicio: 1, percentual: "3.43", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 2, percentual: "3.20", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 3, percentual: "3.01", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 4, percentual: "2.94", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 5, percentual: "2.72", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 6, percentual: "2.28", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 7, percentual: "0.84", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 8, percentual: "0.59", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 9, percentual: "0.59", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 10, percentual: "0.29", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 11, percentual: "0.00", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2019, mes_inicio: 12, percentual: "0.14", fonte: "Portaria ME 9/2019 DOU 16/01/2019 p.25" },
    ReajusteManutencao { ano: 2020, mes_inicio: 1, percentual: "4.48", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 2, percentual: "4.11", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 3, percentual: "3.55", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 4, percentual: "2.76", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 5, percentual: "2.14", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 6, percentual: "1.99", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 7, percentual: "1.98", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 8, percentual: "1.88", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 9, percentual: "1.76", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 10, percentual: "1.81", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 11, percentual: "1.77", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2020, mes_inicio: 12, percentual: "1.22", fonte: "Portaria SEPRT 914/2020 DOU 14/01/2020 p.7" },
    ReajusteManutencao { ano: 2021, mes_inicio: 1, percentual: "5.45", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 2, percentual: "5.25", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 3, percentual: "5.07", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 4, percentual: "4.88", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 5, percentual: "5.12", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 6, percentual: "5.39", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 7, percentual: "5.07", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 8, percentual: "4.61", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 9, percentual: "4.23", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 10, percentual: "3.34", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 11, percentual: "2.42", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2021, mes_inicio: 12, percentual: "1.46", fonte: "Portaria SEPRT/ME 477/2021 DOU 13/01/2021 p.23" },
    ReajusteManutencao { ano: 2022, mes_inicio: 1, percentual: "10.16", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 2, percentual: "9.86", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 3, percentual: "8.97", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 4, percentual: "8.04", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 5, percentual: "7.63", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 6, percentual: "6.61", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 7, percentual: "5.97", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 8, percentual: "4.90", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 9, percentual: "3.99", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 10, percentual: "2.75", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 11, percentual: "1.58", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2022, mes_inicio: 12, percentual: "0.73", fonte: "Portaria Interministerial MTP/ME 12/2022 DOU 20/01/2022 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 1, percentual: "5.93", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 2, percentual: "5.23", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 3, percentual: "4.19", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 4, percentual: "2.43", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 5, percentual: "1.38", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 6, percentual: "0.93", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 7, percentual: "0.30", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 8, percentual: "0.91", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 9, percentual: "1.22", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 10, percentual: "1.55", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 11, percentual: "1.07", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2023, mes_inicio: 12, percentual: "0.69", fonte: "Portaria Interministerial MPS/MF 26/2023 DOU 11/01/2023 p.1" },
    ReajusteManutencao { ano: 2024, mes_inicio: 1, percentual: "3.71", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 2, percentual: "3.23", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 3, percentual: "2.44", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 4, percentual: "1.79", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 5, percentual: "1.26", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 6, percentual: "0.89", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 7, percentual: "0.99", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 8, percentual: "1.08", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 9, percentual: "0.88", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 10, percentual: "0.77", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 11, percentual: "0.65", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2024, mes_inicio: 12, percentual: "0.55", fonte: "Portaria Interministerial MPS/MF 2/2024 DOU 12/01/2024 p.44-45" },
    ReajusteManutencao { ano: 2025, mes_inicio: 1, percentual: "4.77", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 2, percentual: "4.17", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 3, percentual: "3.34", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 4, percentual: "3.14", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 5, percentual: "2.76", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 6, percentual: "2.29", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 7, percentual: "2.04", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 8, percentual: "1.77", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 9, percentual: "1.91", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 10, percentual: "1.43", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 11, percentual: "0.81", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2025, mes_inicio: 12, percentual: "0.48", fonte: "Portaria Interministerial MPS/MF 6/2025 DOU 13/01/2025 p.50" },
    ReajusteManutencao { ano: 2026, mes_inicio: 1, percentual: "3.90", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 2, percentual: "3.90", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 3, percentual: "2.38", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 4, percentual: "1.86", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 5, percentual: "1.38", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 6, percentual: "1.02", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 7, percentual: "0.79", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 8, percentual: "0.58", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 9, percentual: "0.79", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 10, percentual: "0.27", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 11, percentual: "0.24", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
    ReajusteManutencao { ano: 2026, mes_inicio: 12, percentual: "0.21", fonte: "Portaria Interministerial MPS/MF 13/2026 Anexo I p.4 PDF" },
];

/// Reajusta uma renda calculada na `data_ref` (ex.: 13/11/2019) até a `dib` pelos
/// percentuais anuais oficiais de benefícios em manutenção (IN 128/2022, art. 234, II).
/// Retorna `None` quando não há tabela cadastrada para algum ano do intervalo.
pub fn reajustar_renda_manutencao(
    valor_base: Decimal,
    data_ref: &IsoDate,
    dib: &IsoDate,
) -> Option<(Decimal, Vec<(i32, Decimal, Decimal)>)> {
    if dib <= data_ref {
        return Some((to_cents(valor_base), Vec::new()));
    }
    let mut atual = to_cents(valor_base);
    let mut etapas = Vec::new();
    for ano in (data_ref.ano + 1)..=dib.ano {
        let mes_inicio = if ano == data_ref.ano + 1 { data_ref.mes } else { 1 };
        let item = REAJUSTES_BENEFICIOS
            .iter()
            .find(|r| r.ano == ano && r.mes_inicio == mes_inicio)?;
        let pct = Decimal::from_str(item.percentual).ok()?;
        let fator = Decimal::ONE + (pct / Decimal::from(100));
        atual = to_cents(atual * fator);
        etapas.push((ano, pct, atual));
    }
    Some((atual, etapas))
}
