import {useMemo,useState} from 'react';
import type {Calculation, Workspace} from '../model';
import {hasFinancialResult, money} from '../model';
import {api, errorMessage} from '../api';
import {Badge, Button, Modal, Notice, Panel} from './ui';
import type {FabricaSummary} from '../fabrica';
import {buildFabricaInput, difference, localRmi} from '../fabrica';

function when(value: string|undefined|null): string {
  if (!value) return '—';
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('pt-BR');
}
function amount(value: string|undefined|null): string {
  return value ? 'R$ ' + money(String(value)) : '—';
}
function coefficient(value: unknown): string {
  const number = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(number)
    ? (number * 100).toLocaleString('pt-BR', {minimumFractionDigits: 2, maximumFractionDigits: 2}) + '%'
    : '—';
}
function shortHash(value: string|undefined|null): string {
  return value ? value.slice(0, 16) + '…' : '—';
}
function percent(value: string): string {
  const number = Number(value);
  return Number.isFinite(number) ? (number > 0 ? '+' : '') + number.toLocaleString('pt-BR', {minimumFractionDigits: 1, maximumFractionDigits: 1}) + '%' : value;
}
function deltaMoney(value: bigint): string {
  const negative = value < 0n;
  const absolute = negative ? -value : value;
  return (negative ? '-R$ ' : 'R$ ') + money((Number(absolute) / 100).toFixed(2));
}
export function FabricaCheck({
  draft,
  workspace,
  onChange,
}: {
  draft: Calculation;
  workspace: Workspace;
  onChange: (next: Calculation) => void;
}) {
  const key = (workspace.settings.fabrica_api_key ?? '').trim();
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const built = useMemo(() => buildFabricaInput(draft), [draft]);
  const summary = (draft.fabrica ?? null) as FabricaSummary|null;
  const local = localRmi(draft.result);
  const stale = Boolean(
    summary?.inputFingerprint
    && built.input
    && summary.inputFingerprint !== JSON.stringify(built.input),
  );
  const blocked = !hasFinancialResult(draft);

  async function send() {
    if (!built.input || !key) return;
    setBusy(true);
    setError('');
    try {
      const result = await api.conferirFabricaRmi(built.input, key);
      onChange({
        ...draft,
        fabrica: { ...result, inputFingerprint: JSON.stringify(built.input) } as unknown as Calculation['fabrica'],
      });
      setConfirm(false);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  return <>
    {summary
      ? <ProviderSummary
          summary={summary}
          local={local}
          stale={stale}
          busy={busy}
          error={error}
          onAgain={() => setConfirm(true)}
          onClear={() => onChange({ ...draft, fabrica: null })}
        />
      : <Panel
          title="Conferência na Fábrica de Cálculos/TRF3"
          subtitle="Provedor externo opcional para comparar a RMI. Não substitui o motor local, a revisão nem a perícia."
        >
          <p className="form-note">
            A conferência envia somente data de nascimento, sexo, DIB, espécie e os
            períodos/salários revisados. Não envia nome, CPF, NIT, CNIS, PDF,
            anotações ou a chave dentro do corpo. O resultado do provedor fica
            registrado com fonte, horário e hash.
          </p>
          {blocked && <Notice>Execute o motor local com um resultado financeiro antes de conferir online. Um bloqueio local não é contornado pela Fábrica.</Notice>}
          {!key && <Notice>Informe a chave da API em <strong>Configurações → Fábrica de Cálculos</strong>. A chave fica na base local cifrada.</Notice>}
          {built.issues.length > 0 && <Notice><strong>Antes de enviar:</strong><ul className="review-list">{built.issues.slice(0, 8).map(issue => <li key={issue}>{issue}</li>)}</ul></Notice>}
          {error && <Notice tone="error">{error}</Notice>}
          <div className="form-actions" data-ui-actions>
            <Button primary disabled={busy || blocked || !key || !built.input} onClick={() => setConfirm(true)}>
              {busy ? <><span className="spinner"/>Conferindo...</> : 'Conferir na Fábrica de Cálculos'}
            </Button>
            <span className="form-note">{built.input ? `${built.input.periodos.length} período(s) e ${built.input.periodos.reduce((total, period) => total + period.salarios.length, 0)} salário(s) revisados.` : 'Resolva as pendências acima.'}</span>
          </div>
        </Panel>}
    {confirm && built.input && <Modal
      title="Enviar dados revisados à Fábrica de Cálculos?"
      subtitle="Confira exatamente o que sai do computador. Nome, CPF, NIT, CNIS, PDF e anotações ficam fora."
      onClose={() => setConfirm(false)}
    >
      <div className="form-grid" data-ui-stack>
        <div><span className="form-note">Espécie</span><strong>{built.input.especie}</strong></div>
        <div><span className="form-note">DIB / fato gerador</span><strong>{built.input.dataApuracao}{built.input.fatoGerador ? ' · ' + built.input.fatoGerador : ''}</strong></div>
        <div><span className="form-note">Períodos</span><strong>{built.input.periodos.length}</strong></div>
        <div><span className="form-note">Salários</span><strong>{built.input.periodos.reduce((total, period) => total + period.salarios.length, 0)}</strong></div>
      </div>
      <p className="form-note">A chave vai apenas no cabeçalho <code>x-api-key</code> para o endpoint oficial. A resposta completa não é gravada; só o resumo, a fonte e o hash.</p>
      {error && <Notice tone="error">{error}</Notice>}
      <div className="form-actions" data-ui-actions>
        <Button onClick={() => setConfirm(false)}>Cancelar</Button>
        <Button primary disabled={busy} onClick={() => void send()}>{busy ? <><span className="spinner"/>Enviando...</> : 'Enviar e conferir'}</Button>
      </div>
    </Modal>}
  </>;
}
function ProviderSummary({
  summary,
  local,
  stale,
  busy,
  error,
  onAgain,
  onClear,
}: {
  summary: FabricaSummary;
  local: { rmi: string|null; corrected: boolean };
  stale: boolean;
  busy: boolean;
  error: string;
  onAgain: () => void;
  onClear: () => void;
}) {
  const provider = summary.provider;
  const best = provider.beneficios?.[0];
  const providerRmi = provider.maiorRmi;
  const hasRmi = providerRmi !== '0.00' && Boolean(best);
  const diff = hasRmi ? difference(local.rmi, providerRmi) : null;
  return <Panel
    title="Conferência na Fábrica de Cálculos/TRF3"
    subtitle="Provedor externo consultado com os dados revisados. O resultado local não é substituído."
  >
    <div className="head-actions" data-ui-actions>
      <Badge tone="review">Provedor externo</Badge>
      <Badge>Fonte e hash registrados</Badge>
    </div>
    {stale && <Notice tone="error">Os dados do caso mudaram depois desta conferência. Execute novamente para comparar a mesma hipótese.</Notice>}
    {error && <Notice tone="error">{error}</Notice>}
    {!hasRmi && <Notice><strong>A Fábrica não apurou RMI para esta hipótese.</strong> O provedor pode exigir uma declaração de DII/incapacidade ou um requisito que não foi enviado. O motor local e a revisão continuam valendo; nenhum valor foi inventado.</Notice>}
    {best?.alerta && <Notice>{best.alerta}</Notice>}
    {provider.conclusao?.completo === false && <Notice>A Fábrica interrompeu o cálculo{provider.conclusao.motivoInterrupcao ? ': ' + provider.conclusao.motivoInterrupcao : '.'} O valor exibido é o que ela retornou.</Notice>}
    {hasRmi && <div className="result-strip">
      <div><span>RMI do provedor</span><strong>{amount(providerRmi)}</strong></div>
      <div><span>Regra</span><strong>{best?.fundamento ?? best?.chaveFundamento ?? '—'}</strong></div>
      <div><span>Salário de benefício</span><strong>{amount(best?.dadosRmi?.salarioBeneficio)}</strong></div>
      <div><span>Coeficiente</span><strong>{coefficient(best?.dadosRmi?.coeficiente)}</strong></div>
    </div>}
    {(provider.beneficios?.length ?? 0) > 0 && <div className="table-wrap"><table className="data-table">
      <thead><tr><th>Benefício</th><th>Fundamento</th><th className="num">RMI</th><th className="num">SB</th><th className="num">Coef.</th><th>Competências</th></tr></thead>
      <tbody>{provider.beneficios.map((benefit, index) => <tr key={index}>
        <td><strong>{benefit.descricao ?? 'Benefício'}</strong><div className="secondary-line">espécie {benefit.especie ?? '—'}</div></td>
        <td>{benefit.fundamento ?? benefit.chaveFundamento ?? '—'}<div className="secondary-line">{benefit.alerta ?? ''}</div></td>
        <td className="num">{amount(benefit.dadosRmi?.rmi)}</td>
        <td className="num">{amount(benefit.dadosRmi?.salarioBeneficio)}</td>
        <td className="num">{coefficient(benefit.dadosRmi?.coeficiente)}</td>
        <td>{benefit.dadosRmi?.competenciaInicial ?? '—'} a {benefit.dadosRmi?.competenciaFinal ?? '—'}<div className="secondary-line">divisor {benefit.dadosRmi?.divisor ?? '—'} · {benefit.dadosRmi?.totalCompetenciasConsideradas ?? '—'} competências</div></td>
      </tr>)}</tbody>
    </table></div>}
    {(best?.requisitos?.length ?? 0) > 0 && <div className="table-wrap"><table className="data-table">
      <thead><tr><th>Requisito</th><th>Resultado do provedor</th><th>Status</th></tr></thead>
      <tbody>{best?.requisitos?.map((requisito, index) => <tr key={index}>
        <td>{requisito.rotulo ?? requisito.nome}</td>
        <td>{requisito.porExtenso ?? `${String(requisito.exigido ?? '—')} / ${String(requisito.apurado ?? '—')}`}{requisito.nota ? <div className="secondary-line">{requisito.nota}</div> : null}</td>
        <td>{requisito.cumprido === true ? 'Cumprido' : requisito.cumprido === false ? 'Não cumprido' : '—'}{requisito.inconsistencia ? ' · inconsistência' : ''}</td>
      </tr>)}</tbody>
    </table></div>}
    <div className="result-strip">
      <div><span>Motor local</span><strong>{local.rmi ? amount(local.rmi) : 'Sem RMI local'}</strong></div>
      <div><span>Correção local</span><strong>{local.corrected ? 'Ativa' : 'Nominal / sem correção'}</strong></div>
      <div><span>Diferença para o provedor</span><strong>{diff ? `${deltaMoney(diff.cents)} (${percent(diff.percent)})` : '—'}</strong></div>
    </div>
    {local.rmi && !local.corrected && <Notice>O resultado local está sem correção monetária; a Fábrica corrige os salários. Ative a correção no editor e execute o motor local para uma comparação direta.</Notice>}
    {local.rmi && local.corrected && !diff && hasRmi && <Notice>Não foi possível comparar os valores; confira os centavos na memória dos dois resultados.</Notice>}
    <details className="details"><summary>Fonte e proveniência</summary>
      <ul className="review-list">
        <li>Endpoint oficial: {summary.source_url}</li>
        <li>Consultado em: {when(summary.retrieved_at)}</li>
        <li>Hash da resposta: {shortHash(summary.response_sha256)}</li>
        <li>Hash do pedido: {shortHash(summary.request_sha256)}</li>
        <li>Tamanho: {(summary.bytes / 1024 / 1024).toFixed(1)} MB · {provider.resultados ?? 0} resultado(s) · {provider.linhasDescartes ?? 0} linha(s) de descarte</li>
        <li>Conclusão do provedor: {provider.conclusao?.completo === false ? 'interrompida' : 'completa'}</li>
      </ul>
    </details>
    <Notice><strong>Limite:</strong> a Fábrica é um provedor externo. O valor depende dos dados enviados e das regras dela; não é certificação de direito nem concessão. O motor local continua experimental e separado. A chave não aparece neste resumo.</Notice>
    <div className="form-actions" data-ui-actions>
      <Button primary disabled={busy} onClick={onAgain}>{busy ? <><span className="spinner"/>Conferindo...</> : 'Conferir novamente'}</Button>
      <Button disabled={busy} onClick={onClear}>Limpar conferência</Button>
      <span className="form-note">A conferência fica no caso; salve o cálculo para guardar a fonte e o hash.</span>
    </div>

  </Panel>;
}


