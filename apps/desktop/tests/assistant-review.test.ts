import {describe,expect,it} from 'vitest';
import {assistantConsideration} from '../src/assistantReview';
import {blankCase,blankInterview,emptyIdentity} from '../src/model';
import type {Calculation,Cnis,ManualLink,ManualSalary} from '../src/model';

function policy(overrides: Record<string, unknown> = {}) {
  return JSON.stringify({
    version:'rgps-core-observational-v1',
    review_evidence:'DOC-SINTETICO-1',
    manual_link_kind:'empregado',
    manual_currency:'BRL',
    native_options:{corrigirSalarios:false,descarte:false,divisorMinimo:false,aplicarTetoVigenteDer:false},
    exclude_imported_source:true,
    source_exclusion_evidence:'DOC-SINTETICO-2',
    ...overrides,
  });
}
function readyCase(): Calculation {
  const c = blankCase();
  c.name = 'Cenário sintético do assistente';
  c.identity = {...emptyIdentity(), full_name:'Pessoa Fictícia de Teste', birth_date:'1960-01-01'};
  c.sex = 'Feminino';
  c.disability = 'Sem deficiência declarada';
  c.dib = '2024-01-01';
  c.assistant = {...blankInterview(), mode:'assisted', current_benefit_received:false, import_requested:false,
    process_kind:'Simulação do benefício', goal:'Calcular um novo benefício', review_confirmed:true,
    completed_steps:['intro','benefit','import','process','goal','parameters','periods','salaries','policy','review']};
  c.options.rule_policy = policy();
  return c;
}
function link(overrides: Partial<ManualLink> = {}): ManualLink {
  return {id:1,employer:'EMPREGADOR FICTÍCIO',start_date:'2020-01-01',end_date:'2023-12-31',regime:'RGPS',indicators:'',confirmed:true,source_review:null,...overrides};
}
function salary(overrides: Partial<ManualSalary> = {}): ManualSalary {
  return {id:2,competence:'2022-01',amount:'2000.00',indicators:'',confirmed:true,...overrides};
}
function cnis(overrides: Partial<Cnis> = {}): Cnis {
  return {
    metadata:{page_count:1,source_sha256:'synthetic-sha',extractor:'synthetic'},
    identity:{...emptyIdentity(), full_name:'Pessoa Fictícia de Teste'},
    salary_entries:[], salary_months:{},
    links:[{sequence:1,raw_employer:'EMPREGADOR FICTÍCIO',affiliation:'01/01/2020',start_date:'2020-01-01',end_date:null,prpps:false,indicators:[],reported_last_salary_month:null,inferred_last_salary_end:null,source:{page:1}}],
    benefits:[], warnings:[], findings:[], ...overrides,
  };
}

describe('assistant data consideration',()=>{
  it('does not call imported source "used" and names the source gate',()=>{
    const c = readyCase();
    c.assistant!.import_requested = true;
    c.cnis = cnis();
    c.options.rule_policy = policy({exclude_imported_source:false});
    const r = assistantConsideration(c);
    expect(r.imported).toMatchObject({present:true,links:1,entries:0});
    expect(r.used).toEqual({links:0,salaries:0,months:0,source:false});
    expect(r.ready).toBe(false);
    expect(r.issues.map(i=>i.code)).toContain('source_gate');
    expect(r.issues.map(i=>i.code)).toContain('links_missing');
    expect(r.issues.map(i=>i.code)).toContain('salaries_missing');
    expect(r.conclusion).toContain('Ainda não é possível calcular');
  });

  it('marks a fully reviewed manual hypothesis ready and reports what the engine receives',()=>{
    const c = readyCase();
    c.manual_links = [link()];
    c.manual_salaries = [salary()];
    const r = assistantConsideration(c);
    expect(r.ready).toBe(true);
    expect(r.used).toEqual({links:1,salaries:1,months:1,source:false});
    expect(r.adjusted).toMatchObject({manualLinks:1,selectedLinks:1,confirmedLinks:1,excludedLinks:0,salaries:1,confirmedSalaries:1,sourceExcluded:true,sourceExclusionDocumented:true});
    expect(r.conclusion).toContain('prontos para o motor');
  });

  it('keeps an unapplied contribution ledger decisive instead of pretending the edit was used',()=>{
    const c = readyCase();
    c.manual_links = [link()];
    c.manual_salaries = [salary()];
    c.contribution_review = {version:1,source_sha256:null,records:[],history:[],applied:false};
    const r = assistantConsideration(c);
    expect(r.ready).toBe(false);
    expect(r.issues.map(i=>i.code)).toContain('contribution_unapplied');
    expect(r.conclusion).toContain('Livro de contribuições');
  });

  it('refuses PRPPS, unconfirmed and overlapping links',()=>{
    const c = readyCase();
    c.manual_links = [
      link({id:1,regime:'PRPPS',confirmed:false,end_date:'2022-12-31'}),
      link({id:2,employer:'OUTRO FICTÍCIO',start_date:'2022-01-01',end_date:'2023-12-31'}),
    ];
    c.manual_salaries = [salary()];
    const r = assistantConsideration(c);
    expect(r.ready).toBe(false);
    const codes = r.issues.map(i=>i.code);
    expect(codes).toContain('links_regime');
    expect(codes).toContain('links_prpps');
    expect(codes).toContain('links_unconfirmed');
    expect(codes).toContain('links_overlap');
  });

  it('refuses duplicate competences and salaries outside the reviewed link',()=>{
    const c = readyCase();
    c.manual_links = [link()];
    c.manual_salaries = [salary(),salary({id:3}),salary({id:4,competence:'2019-12'})];
    const r = assistantConsideration(c);
    const codes = r.issues.map(i=>i.code);
    expect(codes).toContain('salaries_duplicate');
    expect(codes).toContain('salaries_outside_links');
  });

  it('lists source pendings without letting them block when the source is excluded',()=>{
    const c = readyCase();
    c.cnis = cnis({links:[{sequence:1,raw_employer:'EMPREGADOR FICTÍCIO',affiliation:null,start_date:'2020-01-01',end_date:null,prpps:true,indicators:['PRPPS'],reported_last_salary_month:null,inferred_last_salary_end:null,source:{page:1}}]});
    c.manual_links = [link()];
    c.manual_salaries = [salary()];
    const r = assistantConsideration(c);
    expect(r.ready).toBe(true);
    expect(r.sourceNotes.map(n=>n.code)).toEqual(expect.arrayContaining(['source_prpps','source_end_missing']));
  });

  it('shows the effect of edits and excludes them from the used count',()=>{
    const c = readyCase();
    c.manual_links = [link({id:1}),link({id:2,start_date:'2024-01-01',end_date:'2024-06-30',source_review:{sequence:1,source_sha256:null,excluded:true,history:[]}})];
    c.manual_salaries = [salary()];
    const r = assistantConsideration(c);
    expect(r.adjusted.excludedLinks).toBe(1);
    expect(r.adjusted.selectedLinks).toBe(1);
    expect(r.effect).toContain('excluído');
    expect(r.ready).toBe(true);
    expect(r.used.links).toBe(1);
  });

  it('never turns a blocked result into a financial conclusion',()=>{
    const c = readyCase();
    c.manual_links = [link()];
    c.manual_salaries = [salary()];
    c.result = {status:'blocked',data:null};
    const r = assistantConsideration(c);
    expect(r.conclusion).toContain('bloqueada');
    expect(r.conclusion).not.toContain('R$');
  });
});
