import {render,screen} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {describe,it,expect} from 'vitest';
import {CnisReview} from '../src/components/CnisReview';
import type {Cnis} from '../src/model';

function fixture():Cnis{return{
  metadata:{page_count:2,source_sha256:null,extractor:'synthetic'},
  identity:{full_name:'Pessoa fictícia',cpf:null,nit:null,birth_date:'1960-01-01',mother_name:null},
  links:[{sequence:1,raw_employer:'Empresa fictícia',start_date:'1999-01-01',end_date:null,prpps:true,indicators:['PRPPS'],reported_last_salary_month:'2000-01',inferred_last_salary_end:{date_candidate:'2000-01-31',requires_confirmation:true},source:{page:1}}],
  salary_entries:Array.from({length:21},(_,index)=>({competence:`2000-${String(index%12+1).padStart(2,'0')}`,amount_centavos:100001+index,indicators:[],link_sequence:1,source:{page:2,line:index+1,index},excluded_from_validated_domain:false})),
  salary_months:{},benefits:[],findings:[],warnings:[],
};}
describe('CNIS source review',()=>{
  it('keeps absent end dates explicit and does not alter the imported source',async()=>{
    const cnis=fixture(),original=JSON.stringify(cnis),user=userEvent.setup();render(<CnisReview cnis={cnis}/>);
    await user.click(screen.getByRole('tab',{name:/Períodos/}));
    expect(screen.getByText('Fim não informado')).toBeInTheDocument();
    expect(screen.getByText(/Não confirma encerramento/)).toBeInTheDocument();
    expect(screen.getByText('RPPS / CTC a conferir')).toBeInTheDocument();
    expect(JSON.stringify(cnis)).toBe(original);
  });
  it('exposes every source row through pagination and supports keyboard tabs',async()=>{
    const user=userEvent.setup();render(<CnisReview cnis={fixture()}/>);
    const first=screen.getByRole('tab',{name:'Dados pessoais'});first.focus();await user.keyboard('{End}');
    expect(screen.getByRole('tab',{name:/Remunerações/})).toHaveFocus();
    expect(screen.getByText('1 a 20 de 21')).toBeInTheDocument();
    await user.click(screen.getByRole('button',{name:'Próxima página'}));
    expect(screen.getByText('21 a 21 de 21')).toBeInTheDocument();expect(screen.getByText('1.000,21')).toBeInTheDocument();
    expect(screen.getByRole('button',{name:'Próxima página'})).toBeDisabled();
  });
});
