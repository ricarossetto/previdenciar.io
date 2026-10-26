use serde_json::json;

fn main() {
    let data = json!({
        "title": "Análise da Situação Atual do Benefício",
        "date": "22/10/2026",
        "identification": { "rows": [
            [["Nome do Segurado", "LORETTA VERCELINO SILVA"], ["Nº de Identificação do Trabalhador (NIT)", "267.12342.28-5"]],
            [["CPF", "111.111.111-11"], ["Data de Nascimento", "10/10/1979"], ["Sexo", "Feminino"], ["Data do Cálculo", "22/05/2026"]],
            [["Espécie do Benefício", "42 - Aposentadoria Por Tempo de Contribuição ou Programada"]],
            [["Regra de Cálculo", "Regra do Cálculo"]],
        ]},
        "highlight": [
            ["Data de aposentadoria", "22/05/2026"],
            ["Valor estimado da aposentadoria", "R$ 3.444,44"],
        ],
        "requirements": [
            {"label": "TEMPO DE CONTRIBUIÇÃO", "value": "22 anos, 1 mês e 4 dias", "status": "Requisito atingido", "ok": true},
            {"label": "IDADE", "value": "46 anos, 7 meses e 12 dias", "status": "Sem idade mínima necessária", "ok": false},
        ],
        "a_pagar": [
            ["Salário de contribuição", "R$ 6.823,19"],
            ["Alíquota", "20%"],
            ["Contribuição mensal (INSS)", "R$ 1.364,64"],
            ["IRRF", "R$ 592,37"],
            ["Nº de contribuições", "0"],
            ["Total de contribuições (INSS + IRRF)", "R$ 0,00"],
        ],
        "a_receber": [
            ["Valor estimado da aposentadoria", "R$ 3.444,44"],
            ["Expectativa de sobrevida", "83 anos, 1 mês e 11 dias"],
            ["Nº estimado de meses a receber", "475"],
            ["IRRF", "R$ 0,00"],
            ["Tempo para repor o investimento", "0 meses"],
            ["Total de benefício a receber", "R$ 1.632.377,53"],
        ],
        "signature": "Ricardo De Luca Rossetto",
    });
    let kind = std::env::args().nth(2).unwrap_or_else(|| "analysis".into());
    match previus_reports::render_typst(&kind, &data) {
        Ok(pdf) => {
            let path = std::env::args().nth(1).unwrap_or_else(|| "typst-analise.pdf".into());
            std::fs::write(&path, &pdf).expect("write");
            println!("{{\"status\":\"ok\",\"bytes\":{},\"path\":\"{}\"}}", pdf.len(), path);
        }
        Err(e) => {
            eprintln!("ERRO: {e}");
            std::process::exit(1);
        }
    }
}
