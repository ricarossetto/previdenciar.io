import { useId, useState } from 'react';
import type { Calculation } from '../model';
import { api, errorMessage } from '../api';
import { Badge, Button, Field, Notice } from './ui';

/// Prepara a hipótese a partir do CNIS e calcula em um passo. O produto absorve
/// as decisões de política; o operador fornece só a referência da conferência.
/// O formulário fica aberto na própria conferência: sem modal, o caminho de
/// saída e o que o clique faz ficam visíveis para quem nunca usou o programa.
export function PrepareFromCnis({ draft, onResult, onNotify }: {
  draft: Calculation;
  onResult: (prepared: Calculation) => void;
  onNotify: (message: string, error?: boolean) => void;
}) {
  const titleId = useId();
  const [reference, setReference] = useState('');
  const [notes, setNotes] = useState<string[]>([]);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const hasSource = Boolean(draft.cnis);
  const hasReview = Boolean(draft.contribution_review) || draft.manual_links.some(link => link.source_review);

  async function run() {
    if (!reference.trim()) {
      setError('Escreva a referência documental da conferência antes de preparar a hipótese.');
      return;
    }
    setBusy(true);
    setError('');
    try {
      const prepared = await api.prepareFromCnis(draft, reference.trim());
      setNotes(prepared.notes);
      const result = await api.calculate(prepared.calculation);
      const blocked = (result as Record<string, unknown>)?.status === 'blocked';
      onResult({
        ...prepared.calculation,
        result,
        status: blocked ? 'Execução bloqueada; dados pendentes' : 'Simulação técnica calculada',
      });
      onNotify(
        blocked
          ? 'Hipótese preparada, mas o motor ainda bloqueia. Confira as pendências no resultado.'
          : 'Hipótese preparada a partir do CNIS e calculada. Confira as notas e a memória.',
        blocked,
      );
    } catch (caught) {
      const message = errorMessage(caught);
      setError(message);
      onNotify(message, true);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="conference-prepare" aria-labelledby={titleId} data-ui-container>
      <header className="conference-prepare__head">
        <div>
          <span className="eyebrow">Antes de calcular, 2 de 2</span>
          <h3 id={titleId}>Montar a hipótese a partir do CNIS</h3>
        </div>
        <Badge tone={hasSource ? 'review' : 'neutral'}>{hasSource ? 'CNIS aplicado, ainda não usado' : 'Sem CNIS aplicado'}</Badge>
      </header>
      <p className="conference-prepare__intro">
        O CNIS é um documento conferido, não a entrada do motor. Um clique revisa a cópia, registra a
        referência e calcula. Você não precisa excluir a fonte nem digitar vínculo e remuneração à mão.
      </p>
      <ol className="conference-prepare__steps">
        <li><strong>Exclui a fonte como entrada</strong> e registra a sua referência como justificativa, sem alterar o PDF.</li>
        <li><strong>Monta vínculos e remunerações</strong> a partir do que foi lido: vínculo PRPPS fica fora, data final ausente é fechada pelo último salário e remunerações concomitantes do RGPS são somadas.</li>
        <li><strong>Aplica a política</strong> de correção pelo INPC até a DIB, sem descarte, e envia a hipótese ao motor.</li>
      </ol>
      <Field
        required
        label="Referência documental da conferência"
        value={reference}
        onChange={value => { setReference(value); if (error) setError(''); }}
        placeholder="Documento e data da conferência, sem dados pessoais"
      />
      <p className="form-note">A referência fica registrada nas notas e na política do cálculo. Ela não comprova o direito ao benefício nem substitui a revisão profissional.</p>
      {hasReview && <Notice tone="error">Este cálculo já tem vínculos ou remunerações revisados. Preparar a partir do CNIS substitui esses registros pelos dados da fonte. Para preservar o livro atual, cancele e crie outro cálculo.</Notice>}
      {!hasSource && <Notice>Importe o CNIS na etapa de situação e origem para usar este caminho. Sem a fonte, confira períodos e remunerações nas abas abaixo.</Notice>}
      {error && <Notice tone="error">{error}</Notice>}
      <div className="form-actions" data-ui-actions>
        <Button primary disabled={!hasSource || busy} onClick={() => void run()}>
          {busy ? 'Preparando e calculando...' : 'Preparar e calcular'}
        </Button>
      </div>
      {notes.length > 0 && <ul className="review-list" aria-label="Notas da preparação">{notes.map(note => <li key={note}>{note}</li>)}</ul>}
    </section>
  );
}

