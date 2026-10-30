import { useEffect, useMemo, useRef, useState } from 'react';
import { api, errorMessage } from '../api';
import type { AiDataStatus, KnowledgeBlock, KnowledgeHit, KnowledgeOutcome } from '../api';
import { Badge, Button, Icon, Panel } from './ui';

const BLOCKS: { id: KnowledgeBlock; label: string; short: string; hint: string; placeholder: string }[] = [
  { id: 'skills', label: 'Habilidades', short: 'H', hint: 'Métodos, estruturas e estilos de peça do catálogo da base de conhecimento.', placeholder: 'Ex.: aposentadoria por tempo de contribuição, PPP, CTC…' },
  { id: 'legislation', label: 'Legislação', short: 'L', hint: 'Boletim federal importado, com ficha, categoria e link oficial.', placeholder: 'Ex.: Lei 8.213, aposentadoria, competência…' },
  { id: 'natjus', label: 'NatJus', short: 'N', hint: 'Pareceres técnicos, metadados e texto extraído do PDF quando disponível.', placeholder: 'Ex.: medicamento, tecnologia, doença…' },
  { id: 'jurisprudence', label: 'Jurisprudência', short: 'J', hint: 'Ementas capturadas, com tribunal, processo, relator e link oficial.', placeholder: 'Ex.: coisa julgada, erro médico, responsabilidade…' },
];

function blockLabel(block: KnowledgeBlock): string {
  return BLOCKS.find((item) => item.id === block)?.label ?? block;
}
function fieldText(value: unknown): string {
  return typeof value === 'string' ? value : '';
}
function hitSubtitle(hit: KnowledgeHit): string {
  return hit.tipo || hit.status || hit.identificacao || '';
}

export function KnowledgePage({ notify }: { notify: (message: string, error?: boolean) => void }) {
  const [block, setBlock] = useState<KnowledgeBlock>('skills');
  const [status, setStatus] = useState<AiDataStatus | null>(null);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<KnowledgeHit[]>([]);
  const [searching, setSearching] = useState(false);
  const [selected, setSelected] = useState<KnowledgeOutcome | null>(null);
  const [selectedHit, setSelectedHit] = useState<KnowledgeHit | null>(null);
  const [reading, setReading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [includePdf, setIncludePdf] = useState(true);
  const [notice, setNotice] = useState<{ message: string; error: boolean } | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const current = useMemo(() => BLOCKS.find((item) => item.id === block) ?? BLOCKS[0], [block]);

  useEffect(() => {
    void api.aiDataStatus().then(setStatus).catch((error) => setNotice({ message: errorMessage(error), error: true }));
  }, []);

  async function refreshStatus() {
    try { setStatus(await api.aiDataStatus()); }
    catch (error) { setNotice({ message: errorMessage(error), error: true }); }
  }
  async function search() {
    const value = query.trim();
    if (!value) { setResults([]); setSelected(null); return; }
    setSearching(true);
    setSelected(null);
    setSelectedHit(null);
    try {
      const outcome = await api.aiKnowledgeSearch(block, value, 8);
      const hits = Array.isArray(outcome.value.resultados) ? outcome.value.resultados : [];
      setResults(hits);
      setNotice(hits.length ? null : { message: `Nenhum resultado em ${current.label}. Tente outros termos ou confira a importação.`, error: false });
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
    finally { setSearching(false); }
  }
  async function open(hit: KnowledgeHit) {
    setReading(true);
    setSelectedHit(hit);
    try {
      const outcome = await api.aiKnowledgeRead(block, hit.id);
      setSelected(outcome);
      setNotice(null);
    } catch (error) { setSelected(null); setNotice({ message: errorMessage(error), error: true }); }
    finally { setReading(false); }
  }
  async function importBase() {
    setImporting(true);
    try {
      const summary = await api.aiImportData(includePdf);
      if (!summary) { setNotice({ message: 'Nenhuma pasta foi selecionada.', error: false }); return; }
      await refreshStatus();
      setQuery('');
      setResults([]);
      setSelected(null);
      setNotice({ message: `Base atualizada: ${summary.skills} habilidades, ${summary.legislation} normas, ${summary.natjus} NatJus (${summary.natjusText} com texto) e ${summary.jurisprudence} ementas.`, error: false });
      notify('Base de conhecimento atualizada.');
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
    finally { setImporting(false); }
  }
  function switchBlock(next: KnowledgeBlock) {
    setBlock(next);
    setQuery('');
    setResults([]);
    setSelected(null);
    setSelectedHit(null);
    setNotice(null);
    requestAnimationFrame(() => inputRef.current?.focus());
  }
  function copy(text: string, label: string) {
    void navigator.clipboard?.writeText(text).then(() => notify(label)).catch(() => notify('Não foi possível copiar.', true));
  }

  const content = selected ? (fieldText(selected.value.conteudo) || fieldText(selected.value.ementa) || '') : '';
  const readTitle = selected ? (fieldText(selected.value.titulo) || selectedHit?.titulo || 'Fonte') : '';
  const readUrl = selected ? (fieldText(selected.value.url) || selected.sources[0]?.url || '') : '';
  const sourceKind = selected ? fieldText(selected.value.fonte) : '';
  const total = status ? status.skills + status.legislation + status.natjus + status.jurisprudence : 0;
  return <div className="knowledge-page">
    <div className="knowledge-summary">
      <div><strong>{status?.skills ?? 0}</strong><span>habilidades</span></div>
      <div><strong>{status?.legislation ?? 0}</strong><span>normas</span></div>
      <div><strong>{status?.natjus ?? 0}</strong><span>pareceres NatJus</span><em>{status?.natjusText ?? 0} com texto extraído</em></div>
      <div><strong>{status?.jurisprudence ?? 0}</strong><span>ementas</span></div>
      <div className="knowledge-summary-status"><Badge tone={status?.ready ? 'good' : 'review'}>{status?.ready ? 'Base importada' : 'Base vazia'}</Badge><small>{total} registros locais</small></div>
    </div>

    <div className="knowledge-shell">
      <header className="knowledge-head">
        <div>
          <span className="eyebrow">Base de conhecimento · blocos locais</span>
          <h2>Pesquise antes de pedir ao modelo</h2>
          <p>{current.hint} A busca acontece no seu computador; nada aqui é enviado a um provedor.</p>
        </div>
        <div className="knowledge-import">
          <label className="checkline"><input type="checkbox" checked={includePdf} onChange={(event) => setIncludePdf(event.target.checked)}/>PDFs NatJus + texto</label>
          <Button primary disabled={importing} onClick={() => void importBase()}>{importing ? 'Importando…' : 'Importar / atualizar base'}</Button>
        </div>
      </header>

      <div className="knowledge-tabs" role="tablist" aria-label="Blocos da base de conhecimento">
        {BLOCKS.map((item) => <button key={item.id} role="tab" aria-selected={block === item.id} className={block === item.id ? 'active' : ''} onClick={() => switchBlock(item.id)}>
          <span className="knowledge-tab-ref">{item.short}</span>{item.label}
          <small>{item.id === 'skills' ? status?.skills : item.id === 'legislation' ? status?.legislation : item.id === 'natjus' ? status?.natjus : status?.jurisprudence}</small>
        </button>)}
      </div>

      <div className="knowledge-searchbar">
        <Icon name="search" size={17}/>
        <input ref={inputRef} value={query} onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); void search(); } }} placeholder={current.placeholder} aria-label={`Pesquisar em ${current.label}`}/>
        <Button primary disabled={searching || !query.trim()} onClick={() => void search()}>{searching ? 'Buscando…' : 'Buscar'}</Button>
      </div>

      {notice && <div className={`knowledge-notice ${notice.error ? 'is-error' : ''}`} role={notice.error ? 'alert' : 'status'}><Icon name={notice.error ? 'shield' : 'check'} size={16}/><span>{notice.message}</span><button type="button" className="iconbtn" aria-label="Fechar aviso" onClick={() => setNotice(null)}><Icon name="close" size={14}/></button></div>}

      <div className={`knowledge-results-layout ${selected ? 'has-read' : ''}`}>
        <section className="knowledge-results" aria-live="polite">
          {results.length === 0 && !searching ? <div className="knowledge-empty">
            <span className="knowledge-empty-icon"><Icon name="library" size={26}/></span>
            <h3>{query.trim() ? 'Nenhum resultado' : `Pesquise em ${current.label.toLowerCase()}`}</h3>
            <p>{query.trim() ? 'Tente termos mais gerais ou confira se este bloco foi importado.' : 'A base é consultada localmente. Os resultados mostram o rótulo, a identificação e um trecho para conferência.'}</p>
            {!status?.ready && <p className="form-note">A base ainda não foi importada. Use o botão acima ou o importador do Copiloto.</p>}
          </div> : results.map((hit) => <button key={hit.ref + hit.id} type="button" className={`knowledge-hit ${selectedHit?.id === hit.id ? 'active' : ''}`} onClick={() => void open(hit)}>
            <span className="knowledge-hit-ref">{hit.ref}</span>
            <span className="knowledge-hit-copy"><strong>{hit.titulo}</strong>{hitSubtitle(hit) && <small>{hitSubtitle(hit)}</small>}{hit.trecho && <em>{hit.trecho}</em>}</span>
            <Icon name="arrow" size={16}/>
          </button>)}
        </section>

        {selected && <article className="knowledge-read" aria-label={`Leitura de ${readTitle}`}>
          <header>
            <div>
              <span className="eyebrow">{blockLabel(block)}</span>
              <h3>{readTitle}</h3>
              {sourceKind && <Badge>{sourceKind === 'pdf' ? 'Texto do PDF' : sourceKind === 'ficha' ? 'Ficha local' : sourceKind}</Badge>}
            </div>
            <div className="knowledge-read-actions">
              {readUrl && <Button onClick={() => copy(readUrl, 'Link copiado.')}>Copiar link</Button>}
              <Button onClick={() => { setSelected(null); setSelectedHit(null); }}>Fechar</Button>
            </div>
          </header>
          <div className="knowledge-read-meta">
            {selectedHit && <span>Ref. {selectedHit.ref} · id {selectedHit.id}</span>}
            {readUrl && <code>{readUrl}</code>}
          </div>
          {reading ? <div className="knowledge-reading"><span className="spinner"/>Lendo a fonte…</div> : content ? <pre>{content}</pre> : <p className="form-note">A fonte não retornou texto legível. Consulte o link oficial.</p>}
          <p className="form-note">Conteúdo local para conferência. Não substitui a fonte oficial, a vigência nem a revisão profissional.</p>
        </article>}
      </div>
    </div>

    <Panel title="O que cada bloco contém" subtitle="A base é uma cópia local dos extratos públicos da auditoria.">
      <div className="knowledge-cards">
        {BLOCKS.map((item) => <div key={item.id}><span className="knowledge-tab-ref">{item.short}</span><div><strong>{item.label}</strong><p>{item.hint}</p></div></div>)}
      </div>
    </Panel>
  </div>;
}
