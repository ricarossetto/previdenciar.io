import { describe, expect, it } from 'vitest';
import { atriumBirthDate, caseNotesForAtrium, importMessage, mergeAtriumImport } from '../src/components/AtriumSync';
import type { AtriumContact, AtriumProcess } from '../src/api';
import { blankCase, blankClient, blankWorkspace, type Workspace } from '../src/model';

const contact = (over: Partial<AtriumContact>): AtriumContact => ({
  id: 'c1', name: 'Cliente Sintético', cpf: '', email: '', phone: '', birth_date: '', sex: '', notes: '', ...over,
});
const process = (over: Partial<AtriumProcess>): AtriumProcess => ({
  id: 'p1', number: '', title: '', client: '', contact_id: 'c1', nb: '', phase: '', court: '', notes: '', ...over,
});

function baseWithClient(): Workspace {
  const w = blankWorkspace();
  w.clients.push({ ...blankClient(), id: 1, name: 'Maria Sintética', cpf: '000.000.001-91', phone: '(51) 90000-0001' });
  w.next_id = 2;
  return w;
}

describe('importação do Atrium', () => {
  it('converte a data do Atrium para o formato do cadastro', () => {
    expect(atriumBirthDate('1965-01-20')).toBe('20/01/1965');
    expect(atriumBirthDate('20/01/1965')).toBe('20/01/1965');
    expect(atriumBirthDate('janeiro')).toBe('');
  });

  it('cria cliente novo com o vínculo e caso em rascunho para o processo', () => {
    const { next, summary } = mergeAtriumImport(blankWorkspace(), [
      contact({ cpf: '000.000.003-53', birth_date: '1970-03-02', sex: 'Feminino' }),
    ], [process({ title: 'Aposentadoria por idade', number: '5000000-00.2026.4.04.7100', nb: '123' })]);
    expect(summary).toEqual({ createdClients: 1, updatedClients: 0, createdCases: 1, skippedCases: 0 });
    const client = next.clients[0];
    expect(client).toMatchObject({ name: 'Cliente Sintético', birth_date: '02/03/1970', sex: 'Feminino', atrium_id: 'c1' });
    const created = next.calculations[0];
    expect(created).toMatchObject({ name: 'Aposentadoria por idade', client_id: client.id, atrium_id: 'p1', result: null, status: blankCase().status });
    expect(created.identity).toMatchObject({ full_name: 'Cliente Sintético', cpf: '000.000.003-53', birth_date: '1970-03-02' });
    expect(created.notes).toContain('Processo: 5000000-00.2026.4.04.7100');
    expect(created.cnis).toBeNull();
    expect(next.next_id).toBeGreaterThan(Math.max(client.id, created.id));
  });

  it('reconhece o cliente pelo CPF, completa só campos vazios e não duplica', () => {
    const { next, summary } = mergeAtriumImport(baseWithClient(), [
      contact({ name: 'Outro Nome', cpf: '00000000191', email: 'maria@example.invalid', phone: '(51) 99999-9999' }),
    ], []);
    expect(summary.createdClients).toBe(0);
    expect(summary.updatedClients).toBe(1);
    expect(next.clients).toHaveLength(1);
    expect(next.clients[0]).toMatchObject({ name: 'Maria Sintética', phone: '(51) 90000-0001', email: 'maria@example.invalid', atrium_id: 'c1' });
  });

  it('não cria o mesmo processo duas vezes', () => {
    const first = mergeAtriumImport(blankWorkspace(), [contact({})], [process({ title: 'Caso' })]).next;
    const { next, summary } = mergeAtriumImport(first, [contact({})], [process({ title: 'Caso' })]);
    expect(next.clients).toHaveLength(1);
    expect(next.calculations).toHaveLength(1);
    expect(summary).toEqual({ createdClients: 0, updatedClients: 0, createdCases: 0, skippedCases: 1 });
    expect(importMessage(summary)).toBe('Nada novo para trazer: esses clientes e processos já estão na sua base.');
  });

  it('ignora processo de contato que não foi selecionado', () => {
    const { next } = mergeAtriumImport(blankWorkspace(), [contact({})], [process({ id: 'p2', contact_id: 'outro' })]);
    expect(next.calculations).toHaveLength(0);
  });

  it('resume o caso para o Atrium sem dados do CNIS nem valores', () => {
    const c = { ...blankCase(), id: 9, name: 'Caso', status: 'Simulação técnica calculada', result: { rmi: '1234.56' } };
    const notes = caseNotesForAtrium(c, '24/11/2026');
    expect(notes).toBe('Caso do ATRIUM Prev, enviado em 24/11/2026.\nCenário: Simulação de novo benefício\nSituação no ATRIUM Prev: Simulação técnica calculada');
    expect(notes).not.toContain('1234');
  });
});
