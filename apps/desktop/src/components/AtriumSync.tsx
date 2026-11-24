// Troca de clientes e casos com o Atrium (somente no site).
//
// Importar: contatos do Atrium viram clientes; processos ligados a eles viram casos em
// rascunho, sem cálculo. Enviar: o cadastro do cliente e o nome de cada caso vão para o
// Atrium. CNIS, salários, vínculos e resultados nunca saem do previdenciário.
import { useEffect, useMemo, useState } from 'react';
import { errorMessage, type AtriumApi, type AtriumContact, type AtriumProcess } from '../api';
import { allocate, blankCase, putEntity, type Calculation, type Client, type Workspace } from '../model';
import { Badge, Button, Icon, Modal, Notice } from './ui';

const digits = (value: string | null | undefined) => String(value ?? '').replace(/\D+/g, '');

/** Data do Atrium (AAAA-MM-DD ou DD/MM/AAAA) no formato do cadastro (DD/MM/AAAA). */
export function atriumBirthDate(value: string): string {
  const iso = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value.trim());
  if (iso) return `${iso[3]}/${iso[2]}/${iso[1]}`;
  return /^\d{2}\/\d{2}\/\d{4}$/.test(value.trim()) ? value.trim() : '';
}

function atriumSex(value: string): string | null {
  return value === 'Feminino' || value === 'Masculino' ? value : null;
}

/** Cliente da base que corresponde ao contato: pelo vínculo já gravado ou pelo mesmo CPF/CNPJ. */
export function findClientFor(clients: Client[], contact: AtriumContact): Client | undefined {
  const byLink = clients.find(c => c.atrium_id && c.atrium_id === contact.id);
  if (byLink) return byLink;
  const doc = digits(contact.cpf);
  return doc.length >= 11 ? clients.find(c => digits(c.cpf) === doc) : undefined;
}

function processNotes(process: AtriumProcess): string {
  return [
    'Importado do Atrium.',
    process.number && `Processo: ${process.number}`,
    process.nb && `NB: ${process.nb}`,
    process.court && `Órgão: ${process.court}`,
    process.notes,
  ].filter(Boolean).join('\n');
}

export interface AtriumImportSummary { createdClients: number; updatedClients: number; createdCases: number; skippedCases: number }

/**
 * Junta contatos e processos do Atrium na base. Nunca troca dado já preenchido: só
 * completa campos vazios e grava o vínculo. Processo já importado não vira outro caso.
 */
export function mergeAtriumImport(workspace: Workspace, contacts: AtriumContact[], processes: AtriumProcess[]): { next: Workspace; summary: AtriumImportSummary } {
  const next = structuredClone(workspace);
  const summary: AtriumImportSummary = { createdClients: 0, updatedClients: 0, createdCases: 0, skippedCases: 0 };
  const clientIdByContact = new Map<string, number>();
  for (const contact of contacts) {
    const existing = findClientFor(next.clients, contact);
    if (existing) {
      const updated: Client = {
        ...existing,
        atrium_id: existing.atrium_id || contact.id,
        cpf: existing.cpf || contact.cpf,
        email: existing.email || contact.email,
        phone: existing.phone || contact.phone,
        birth_date: existing.birth_date || atriumBirthDate(contact.birth_date),
        sex: existing.sex || atriumSex(contact.sex),
      };
      if (JSON.stringify(updated) !== JSON.stringify(existing)) summary.updatedClients += 1;
      next.clients = putEntity(next.clients, updated);
      clientIdByContact.set(contact.id, existing.id);
      continue;
    }
    const id = allocate(next);
    next.clients.push({
      id,
      name: contact.name,
      cpf: contact.cpf,
      email: contact.email,
      phone: contact.phone,
      birth_date: atriumBirthDate(contact.birth_date),
      sex: atriumSex(contact.sex),
      notes: contact.notes,
      archived: false,
      atrium_id: contact.id,
    });
    next.next_id = id + 1;
    clientIdByContact.set(contact.id, id);
    summary.createdClients += 1;
  }
  for (const process of processes) {
    const clientId = clientIdByContact.get(process.contact_id);
    if (clientId === undefined) continue;
    if (next.calculations.some(c => c.atrium_id === process.id)) {
      summary.skippedCases += 1;
      continue;
    }
    const client = next.clients.find(c => c.id === clientId)!;
    const base = blankCase();
    const id = allocate(next);
    const birth = /^(\d{2})\/(\d{2})\/(\d{4})$/.exec(client.birth_date);
    const caseValue: Calculation = {
      ...base,
      id,
      name: process.title || (process.number ? `Processo ${process.number}` : `Caso de ${client.name}`),
      client_id: clientId,
      identity: { ...base.identity, full_name: client.name, cpf: client.cpf || null, birth_date: birth ? `${birth[3]}-${birth[2]}-${birth[1]}` : null },
      sex: client.sex,
      notes: processNotes(process),
      atrium_id: process.id,
    };
    next.calculations.push(caseValue);
    next.next_id = id + 1;
    summary.createdCases += 1;
  }
  next.next_id = allocate(next);
  return { next, summary };
}

function plural(n: number, one: string, many: string) {
  return `${n} ${n === 1 ? one : many}`;
}

export function importMessage(s: AtriumImportSummary): string {
  const parts: string[] = [];
  if (s.createdClients) parts.push(plural(s.createdClients, 'cliente novo', 'clientes novos'));
  if (s.updatedClients) parts.push(plural(s.updatedClients, 'cliente completado', 'clientes completados'));
  if (s.createdCases) parts.push(plural(s.createdCases, 'caso novo', 'casos novos'));
  if (!parts.length) return 'Nada novo para trazer: esses clientes e processos já estão na sua base.';
  const skipped = s.skippedCases ? ` ${plural(s.skippedCases, 'processo já estava', 'processos já estavam')} na base.` : '';
  return `Importação do Atrium concluída: ${parts.join(', ')}.${skipped}`;
}

export function AtriumImportModal({ atrium, workspace, saving, onImport, onClose }: {
  atrium: AtriumApi;
  workspace: Workspace;
  saving: boolean;
  onImport: (next: Workspace, message: string) => Promise<void>;
  onClose: () => void;
}) {
  const [query, setQuery] = useState('');
  const [contacts, setContacts] = useState<AtriumContact[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [withProcesses, setWithProcesses] = useState(true);
  const [loading, setLoading] = useState(true);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState('');

  async function load(term: string) {
    setLoading(true);
    setError('');
    try {
      setContacts(await atrium.contacts(term));
    } catch (e) {
      setContacts([]);
      setError(errorMessage(e));
    } finally {
      setLoading(false);
    }
  }
  useEffect(() => { void load(''); }, []);

  const inBase = useMemo(() => new Set(contacts.filter(c => findClientFor(workspace.clients, c)).map(c => c.id)), [contacts, workspace.clients]);
  const toggle = (id: string) => setSelected(current => {
    const next = new Set(current);
    if (next.has(id)) next.delete(id); else next.add(id);
    return next;
  });
  const allSelected = contacts.length > 0 && contacts.every(c => selected.has(c.id));

  async function run() {
    const chosen = contacts.filter(c => selected.has(c.id));
    if (!chosen.length) return;
    setWorking(true);
    setError('');
    try {
      const processes = withProcesses ? await atrium.processes(chosen.map(c => c.id)) : [];
      const { next, summary } = mergeAtriumImport(workspace, chosen, processes);
      await onImport(next, importMessage(summary));
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setWorking(false);
    }
  }

  const busy = loading || working || saving;
  return (
    <Modal title="Importar do Atrium" subtitle="Traga clientes do seu escritório no Atrium e, se quiser, os processos ligados a eles." wide onClose={onClose}>
      <form
        className="toolbar"
        onSubmit={e => { e.preventDefault(); void load(query); }}
      >
        <div className="searchbox">
          <Icon name="search" size={17} />
          <input aria-label="Pesquisar no Atrium" value={query} onChange={e => setQuery(e.target.value)} placeholder="Nome, CPF, e-mail ou telefone" />
        </div>
        <Button type="submit" disabled={busy}>{loading ? 'Buscando...' : 'Buscar'}</Button>
      </form>
      {error && <Notice tone="error">{error}</Notice>}
      {loading ? (
        <p className="form-note" role="status">Buscando clientes no Atrium...</p>
      ) : contacts.length ? (
        <div className="table-wrap">
          <table className="data-table">
            <thead>
              <tr>
                <th>
                  <input
                    type="checkbox"
                    aria-label="Selecionar todos"
                    checked={allSelected}
                    onChange={() => setSelected(allSelected ? new Set() : new Set(contacts.map(c => c.id)))}
                  />
                </th>
                <th>Cliente</th>
                <th>CPF / CNPJ</th>
                <th>Na sua base</th>
              </tr>
            </thead>
            <tbody>
              {contacts.map(c => (
                <tr key={c.id}>
                  <td><input type="checkbox" aria-label={`Selecionar ${c.name}`} checked={selected.has(c.id)} onChange={() => toggle(c.id)} /></td>
                  <td>
                    <strong>{c.name}</strong>
                    <div className="secondary-line">{c.email || c.phone || 'Sem contato'}</div>
                  </td>
                  <td>{c.cpf || 'Sem dado'}</td>
                  <td>{inBase.has(c.id) ? <Badge tone="good">Já está</Badge> : <Badge>Novo</Badge>}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : !error ? (
        <p className="form-note">Nenhum cliente encontrado no Atrium. Confira o nome ou o CPF e busque de novo.</p>
      ) : null}
      <label className="checkline">
        <input type="checkbox" checked={withProcesses} onChange={e => setWithProcesses(e.target.checked)} />
        Trazer também os processos desses clientes como casos em rascunho
      </label>
      <p className="form-note">
        Clientes que já estão na base não são duplicados: só os campos vazios são completados. Os casos chegam sem cálculo; o CNIS e os dados do benefício continuam com você.
      </p>
      <div className="row-actions">
        <Button onClick={onClose}>Cancelar</Button>
        <Button primary disabled={busy || selected.size === 0} onClick={() => void run()}>
          {working ? 'Importando...' : selected.size ? `Importar ${plural(selected.size, 'cliente', 'clientes')}` : 'Importar'}
        </Button>
      </div>
    </Modal>
  );
}

/** Resumo enviado ao Atrium nas anotações do processo: nada de CNIS, salários ou valores. */
export function caseNotesForAtrium(c: Calculation, today: string): string {
  return [`Caso do ATRIUM Prev, enviado em ${today}.`, c.scenario && `Cenário: ${c.scenario}`, c.status && `Situação no ATRIUM Prev: ${c.status}`]
    .filter(Boolean).join('\n');
}

function cpfForAtrium(cpf: string): string {
  const n = digits(cpf).length;
  return n === 11 || n === 14 ? cpf : '';
}

export function AtriumSendModal({ atrium, workspace, client, saving, onSent, onClose }: {
  atrium: AtriumApi;
  workspace: Workspace;
  client: Client;
  saving: boolean;
  onSent: (next: Workspace, message: string) => Promise<void>;
  onClose: () => void;
}) {
  const cases = workspace.calculations.filter(c => c.client_id === client.id && !c.archived);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState('');

  async function run() {
    setWorking(true);
    setError('');
    const next = structuredClone(workspace);
    const failures: string[] = [];
    let sentCases = 0;
    try {
      const contact = await atrium.sendContact({
        name: client.name,
        cpf: cpfForAtrium(client.cpf),
        email: client.email,
        phone: client.phone,
        birth_date: client.birth_date,
        sex: client.sex === 'Feminino' || client.sex === 'Masculino' ? client.sex : '',
        notes: client.notes,
      });
      next.clients = next.clients.map(c => (c.id === client.id ? { ...c, atrium_id: contact.id } : c));
      const today = new Date().toLocaleDateString('pt-BR');
      for (const c of cases) {
        try {
          const process = await atrium.sendProcess({
            calculation_id: c.id,
            contact_id: contact.id,
            atrium_id: c.atrium_id ?? '',
            title: c.name.trim() || `Caso de ${client.name}`,
            nb: '',
            notes: caseNotesForAtrium(c, today),
          });
          next.calculations = next.calculations.map(x => (x.id === c.id ? { ...x, atrium_id: process.id } : x));
          sentCases += 1;
        } catch (e) {
          failures.push(`${c.name || 'Caso sem nome'}: ${errorMessage(e)}`);
        }
      }
      const head = contact.created ? 'Cliente cadastrado no Atrium' : 'Cliente já existia no Atrium e foi ligado a este cadastro';
      const tail = cases.length ? ` ${plural(sentCases, 'caso enviado', 'casos enviados')}.` : '';
      await onSent(next, `${head}.${tail}`);
      if (failures.length) setError(`Não foi possível enviar: ${failures.join('; ')}`);
      else onClose();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setWorking(false);
    }
  }

  return (
    <Modal title="Enviar ao Atrium" subtitle={client.name} onClose={onClose}>
      <p>Vão para o Atrium:</p>
      <ul>
        <li>o cadastro do cliente: nome, CPF, e-mail, telefone, nascimento, sexo e anotações;</li>
        <li>{cases.length ? `${plural(cases.length, 'caso', 'casos')} como processo previdenciário: nome do caso, cenário e situação.` : 'nenhum caso, porque o cliente ainda não tem cálculos ativos.'}</li>
      </ul>
      <p className="form-note">CNIS, vínculos, salários e resultados de cálculo não são enviados. Se o CPF já existir no Atrium, o contato de lá é aproveitado e nada dele é sobrescrito.</p>
      {error && <Notice tone="error">{error}</Notice>}
      <div className="row-actions">
        <Button onClick={onClose}>Cancelar</Button>
        <Button primary disabled={working || saving} onClick={() => void run()}>{working ? 'Enviando...' : 'Enviar ao Atrium'}</Button>
      </div>
    </Modal>
  );
}
