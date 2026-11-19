mod common;

use common::{compact, extract, synthetic_case};
use previdenciar_reports::{render_pdf, ReportKind, ReportOptions};

#[test]
fn todos_os_relatorios_reais_ocultam_identificacao_sem_perder_valores() {
    let (caso, mut workspace) = synthetic_case();
    workspace.settings.fabrica_api_key = "CHAVEFABRICAMARCADOR".to_owned();

    // Identificação, nome do cálculo, escritório e rodapé entram na faixa
    // comum de todos os relatórios (layout::Decorator, model::build).
    let universais = [
        "IDENTIDADETESTE",
        "CPFMARCADOR-12345678900",
        "NOMEMARCADOR",
        "ESCRITORIOMARCADOR",
        "RODAPEMARCADOR",
    ];
    // Só o Detailed inclui entrevista, períodos e pendências no renderer.
    let detalhados = ["NOTAMARCADOR", "EMPREGADORMARCADOR", "AVISOFONTEMARCADOR"];
    let chave_fabrica = "CHAVEFABRICAMARCADOR";
    let kinds = [
        ReportKind::Detailed,
        ReportKind::Summary,
        ReportKind::Periods,
        ReportKind::Salaries,
        ReportKind::Memory,
        ReportKind::Transitions,
        ReportKind::Indicators,
        ReportKind::Quality,
        ReportKind::Analysis,
        ReportKind::Roi,
        ReportKind::Regras,
    ];

    for kind in kinds {
        for (include_personal, visivel) in [(true, true), (false, false)] {
            let bytes = render_pdf(
                &caso,
                &workspace,
                ReportOptions::new(kind, include_personal),
                None,
            )
            .unwrap_or_else(|error| panic!("{kind:?}: PDF real falhou: {error}"));
            assert!(bytes.starts_with(b"%PDF-"), "{kind:?}: assinatura PDF");
            let (documento, texto) = extract(&bytes);
            assert!(
                !documento.get_pages().is_empty(),
                "{kind:?}: PDF sem páginas"
            );
            let plano = compact(&texto);
            for marcador in universais {
                let presente = plano.contains(&compact(marcador));
                assert_eq!(
                    presente, visivel,
                    "{kind:?} include_personal={include_personal}: {marcador}"
                );
            }
            if kind == ReportKind::Detailed {
                for marcador in detalhados {
                    let presente = plano.contains(&compact(marcador));
                    assert_eq!(
                        presente, visivel,
                        "{kind:?} include_personal={include_personal}: {marcador}"
                    );
                }
            }
            // A chave da Fábrica nunca é campo de relatório, em nenhum modo.
            assert!(
                !plano.contains(&compact(chave_fabrica)),
                "{kind:?} include_personal={include_personal}: chave no PDF"
            );
            if kind == ReportKind::Salaries {
                // Dado útil continua no relatório privado; só a identificação some.
                assert!(
                    plano.contains("2.000"),
                    "{kind:?} include_personal={include_personal}: salário sintético ausente"
                );
            }
        }
    }
}
