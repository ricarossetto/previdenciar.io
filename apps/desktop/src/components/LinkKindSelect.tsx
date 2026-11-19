import {LINK_KINDS, LINK_KIND_LABELS} from '../assistantReview';

export const SUPPORTED_LINK_KIND_OPTIONS = LINK_KINDS.map(value => ({
  value,
  label: LINK_KIND_LABELS[value] ?? value,
}));

/**
 * Only the identifiers accepted by the local RGPS adapter are selectable.
 * A legacy value is shown but disabled, so it stays visible and pending until
 * a human explicitly chooses a supported category.
 */
export function LinkKindSelect({value,onChange}:{value:string|null;onChange:(value:string|null)=>void}) {
  const legacy = value !== null && value !== '' && !LINK_KINDS.includes(value) ? value : null;
  return <label className="field">
    <span>Categoria dos períodos declarados</span>
    <select className="input" aria-label="Categoria dos períodos declarados" value={value ?? ''} onChange={event => onChange(event.target.value || null)}>
      <option value="">Selecione...</option>
      {SUPPORTED_LINK_KIND_OPTIONS.map(option => <option key={option.value} value={option.value}>{option.label}</option>)}
      {legacy && <option value={legacy} disabled>{legacy}: legado não suportado; revisar</option>}
    </select>
    {legacy && <small className="field-hint">Categoria legada preservada e pendente. Escolha uma categoria suportada; nada foi convertido automaticamente.</small>}
    <small className="field-hint">Rural, tempo especial e servidor sem CTC não são enquadrados por este motor local.</small>
  </label>;
}
