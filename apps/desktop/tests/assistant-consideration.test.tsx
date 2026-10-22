import {describe,expect,it} from 'vitest';
import {render,screen} from '@testing-library/react';
import {AssistantConsiderationPanel} from '../src/components/AssistantConsideration';
import {assistantConsideration} from '../src/assistantReview';
import {blankCase,blankInterview,emptyIdentity} from '../src/model';
import type {Calculation,Cnis} from '../src/model';

function cnis():Cnis{return{
  metadata:{page_count:1,source_sha256:'synthetic',extractor:'synthetic'},
  identity:{full_name:'Pessoa Fictícia',cpf:null,nit:null,birth_date:'1960-01-01',mother_name:null},
  links:[{sequence:1,raw_employer:'EMPREGADOR FICTÍCIO',affiliation:'empregado',start_date:'2020-01-01',end_date:null,prpps:false,indicators:[],reported_last_salary_month:null,inferred_last_salary_end:null,source:{page:1}}],
  salary_entries:[{competence:'2022-01',amount_centavos:200000,indicators:[],link_sequence:1,source:{page:1,line:1,index:1},excluded_from_validated_domain:false}],
  salary_months:{'2022-01':{count:1,amount_sum_centavos:200000,contains_unresolved_prpps:false}},benefits:[],warnings:[],findings:[],
};}
function base():Calculation{
  const c=blankCase();
  c.name='Caso sintético';c.identity={...emptyIdentity(),full_name:'Pessoa Fictícia',birth_date:'1960-01-01'};
  c.sex='Feminino';c.disability='Sem deficiência declarada';c.dib='2024-01-01';
  c.assistant={...blankInterview(),mode:'assisted',current_benefit_received:false,import_requested:true,process_kind:'Simulação do benefício',goal:'Calcular um novo benefício',review_confirmed:false,completed_steps:['intro','benefit','import','process','goal','parameters','periods','salaries','policy']};
  return c;
}
describe('assistant traceability panel',()=>{
  it('shows source, adjustments and the blocked motor input separately',()=>{
    const c=base();c.cnis=cnis();
    render(<AssistantConsiderationPanel consideration={assistantConsideration(c)} onFix={()=>{}}/>);
    expect(screen.getByText('O que entra no cálculo')).toBeInTheDocument();
    expect(screen.getByText('1 vínculo · 1 lançamento')).toBeInTheDocument();
    expect(screen.getByText('0 — execução bloqueada')).toBeInTheDocument();
    expect(screen.getByText('Pendências que impedem calcular')).toBeInTheDocument();
    expect(screen.getAllByText(/Fonte CNIS ainda não excluída/).length).toBeGreaterThan(0);
    expect(screen.getAllByRole('button',{name:/Corrigir:/}).length).toBeGreaterThan(0);
  });
  it('marks a fully reviewed hypothesis ready without rewriting the source',()=>{
    const c=base();
    c.cnis=cnis();
    c.manual_links=[{id:1,employer:'EMPREGADOR FICTÍCIO',start_date:'2020-01-01',end_date:'2023-12-31',regime:'RGPS',indicators:'',confirmed:true,source_review:null}];
    c.manual_salaries=[{id:2,competence:'2022-01',amount:'2000.00',indicators:'',confirmed:true}];
    c.options.rule_policy=JSON.stringify({version:'rgps-core-observational-v1',review_evidence:'DOC-1',manual_link_kind:'empregado',manual_currency:'BRL',native_options:{corrigirSalarios:false,descarte:false,divisorMinimo:false,aplicarTetoVigenteDer:false},exclude_imported_source:true,source_exclusion_evidence:'DOC-2'});
    render(<AssistantConsiderationPanel consideration={assistantConsideration(c)}/>);
    expect(screen.getByText('Pronto para o motor')).toBeInTheDocument();
    expect(screen.getByText('1 vínculo · 1 competência')).toBeInTheDocument();
    expect(screen.queryByText('Pendências que impedem calcular')).not.toBeInTheDocument();
    expect(screen.getByText(/Somente linhas revisadas; a fonte não é reescrita/)).toBeInTheDocument();
  });
});
