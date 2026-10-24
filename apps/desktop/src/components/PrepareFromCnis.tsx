import { useState } from 'react';
import type { Calculation } from '../model';
import { api, errorMessage } from '../api';
import { Button, Field, Modal, Notice } from './ui';

/// Prepara a hipótese a partir do CNIS e calcula em um passo. O produto absorve
/// as decisões de política; o operador fornece só a referência da conferência.
export function PrepareFromCnis({ draft, onResult, onNotify }: {
  draft: Calculation;
  onResult: (prepared: Calculation) => void;
  onNotify: (message: string, error?: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  const [reference, setReference] = useState('');
  const [notes, setNotes] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);

  async function run() {
    if (!reference.trim()) {
      onNotify('Informe a referência documental da conferência.', true);
      return;
    }
    setBusy(true);
    try {
      const prepared = await api.prepareFromCnis(draft, reference.trim());
      setNotes(prepared.notes);
      setOpen(false);
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
    } catch (error) {
      onNotify(errorMessage(error), true);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="pcd-section">
      <h3>Preparar a partir do CNIS</h3>
      <p>Um clique aplica as decisões de revisão e calcula: exclui vínculos PRPPS, fecha vínculo sem data final pelo último salário e soma remunerações concomitantes do RGPS. A fonte permanece intacta e as decisões ficam nas notas.</p>
      <div className="form-actions" data-ui-actions>
        <Button primary disabled={!draft.cnis || busy} onClick={() => setOpen(true)}>
          {busy ? 'Preparando...' : 'Preparar e calcular'}
        </Button>
      </div>
      {notes.length > 0 && <ul className="review-list">{notes.map((note) => <li key={note}>{note}</li>)}</ul>}
      {open && (
        <Modal title="Preparar a partir do CNIS" subtitle="A fonte não é alterada; a hipótese usa uma cópia revisada." onClose={() => setOpen(false)}>
          <Field required label="Referência documental da conferência" value={reference} onChange={setReference} placeholder="Documento e data da conferência, sem dados pessoais" />
          <Notice>Serão aplicadas: exclusão de vínculos PRPPS; fim pelo último salário quando a data final não constar; soma de remunerações concomitantes do RGPS; correção pelo INPC até a DIB, quando houver índice publicado.</Notice>
          <div className="form-actions" data-ui-actions>
            <Button onClick={() => setOpen(false)}>Cancelar</Button>
            <Button primary disabled={busy} onClick={() => void run()}>{busy ? 'Preparando...' : 'Preparar e calcular'}</Button>
          </div>
        </Modal>
      )}
    </div>
  );
}
