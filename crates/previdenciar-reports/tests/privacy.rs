mod common;

use common::{compact, extract, synthetic_case};
use previdenciar_reports::{render_pdf, ReportKind, ReportOptions};

#[test]
fn todos_os_relatorios_reais_ocultam_identificacao_sem_perder_valores() {
    let (caso, mut workspace) = synthetic_case();
    workspace.settings.fabrica_api_key = "CHAVEFABRICAMARCADOR".to_owned();

    let ocultos = [
        "IDENTIDADETESTE",
        "CPFMARCADOR-12345678900",
        "NITMARCADOR-987654321",
        "MÃEMARCADOR",
        "NOMEMARCADOR",
        "NOTAMARCADOR",
        "EMPREGADORMARCADOR",
        "AVISOFONTEMARCADOR",
        "INDICADORPESSOALMARCADOR",
        "ESCRITORIOMARCADOR",
        "RODAPEMARCADOR",
        "CHAVEFABRICAMARCADOR",
    ];
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
        let bytes = render_pdf(
            &caso,
            &workspace,
            ReportOptions::new(kind, false),
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
        for marcador in ocultos {
            assert!(
                !plano.contains(&compact(marcador)),
                "{kind:?}: vazou {marcador}"
            );
        }
        if kind == ReportKind::Salaries {
            assert!(
                plano.contains("2.000"),
                "{kind:?}: salário sintético deveria permanecer"
            );
        }
    }
}
