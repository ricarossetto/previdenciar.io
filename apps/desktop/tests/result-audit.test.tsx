import {render,screen,within} from '@testing-library/react';
import {describe,it,expect} from 'vitest';
import {ResultView} from '../src/components/CaseWorkbench';
import type {Json} from '../src/model';

const result:Json={status:'experimental',data:{pre_1994_wages_excluded:1,source_reconciliation:{input_wages:10,pre_1994_excluded:1,below_minimum_excluded:1,pbc_wages:8,confirmed_links:1},native_result:{resumo:{rmi:'1600.00',salario_beneficio:'2000.00',competencias:8,coeficiente:'80',carencia_meses:8},tempo:{total:{anos:0,meses:8,dias:0},carencia_meses:8,marcos:{ate_ec20:0,ate_ec103:0,apos_ec103:240},vinculos:[{indice:1,tipo:'empregado',inicio:'2024-01-01',fim:'2024-08-31',dias:244,dias_efetivos:244,concomitante:false}],avisos:[]},correcao:{pbc:8,consideradas:8,descartadas:0,media:'2000.00',descarte_aplicado:false,parcelas:[{competencia:'2024-01',salario:'2000.00',corrigido:'2000.00'}],parcelas_descartadas:[],cenarios:[{rotulo:'Sem corte',consideradas:8,descartadas:0,media:'2000.00'}]},transicoes:{regra_escolhida:null,rmi_escolhida:null,tempo_ate_reforma_meses:0,regras:[{nome:'Regra sintética',fundamento:'Teste',elegivel:false,rmi:null,rmi_final:null,requisitos:['Requisito sintético'],faltas:{idadeMeses:12},observacao:null}]},memoria:[]}}};

describe('result audit panels',()=>{
  it('shows PBC, time and transition memory without turning them into a new result',()=>{
    render(<ResultView result={result}/>);
    expect(screen.getByText('PBC e descartes')).toBeInTheDocument();
    expect(screen.getByText('Tempo de contribuição')).toBeInTheDocument();
    expect(screen.getByText('Regras de transição')).toBeInTheDocument();
    expect(screen.getAllByText('Regra sintética').length).toBeGreaterThan(0);
    expect(screen.getByText(/Nenhum salário foi elevado ao mínimo silenciosamente/)).toBeInTheDocument();
    expect(screen.getByText('Sem direito')).toBeInTheDocument();
    expect(screen.getByText('Nenhuma regra está cumprida na DIB.')).toBeInTheDocument();
  });

  it('shows the RMI in hero when an eligible transition rule is chosen',()=>{
    const eligible:Json={
      status:'experimental',
      data:{
        direito_na_dib:true,
        native_result:{
          resumo:{rmi:'1600.00',salario_beneficio:'2000.00',competencias:300,coeficiente:'80',carencia_meses:300},
          transicoes:{
            regra_escolhida:'pontos',
            rmi_escolhida:'1600.00',
            regras:[{id:'pontos',nome:'Pontos',fundamento:'EC 103/2019, art. 15',elegivel:true,rmi_final:'1600.00',requisitos:[],faltas:null,observacao:null}],
          },
          memoria:[],
        },
      },
    };
    render(<ResultView result={eligible}/>);
    expect(screen.queryByText('Sem direito')).not.toBeInTheDocument();
    expect(screen.getAllByText('R$ 1.600,00').length).toBeGreaterThan(0);
  });

  it('shows projected rules panel and renders non-applicable rules clearly without Faltam nada',()=>{
    const projected:Json={
      status:'experimental',
      data:{
        direito_na_dib:false,
        projecao_regras:{
          premissa:'Contribuição contínua a partir de 01/11/2026.',
          regras:[
            {id:'pedagio-100',nome:'Transição por pedágio de 100%',data:'2027-05-19',rmi:'3413.25'},
          ],
        },
        native_result:{
          resumo:{rmi:'3295.72',salario_beneficio:'3433.04',competencias:350,coeficiente:'96',carencia_meses:345},
          transicoes:{
            regra_escolhida:null,
            rmi_escolhida:null,
            regras:[
              {id:'pedagio-50',nome:'Pedágio de 50%',fundamento:'EC 103/2019, art. 17',elegivel:false,rmi:null,rmi_final:null,requisitos:[],faltas:{aplicavel:false},observacao:null},
              {id:'pedagio-100',nome:'Transição por pedágio de 100%',fundamento:'EC 103/2019, art. 20',elegivel:false,rmi:'3433.04',rmi_final:'3433.04',requisitos:[],faltas:{idadeMeses:7,tempoMeses:4},observacao:null},
            ],
          },
          memoria:[],
        },
      },
    };
    render(<ResultView result={projected}/>);
    expect(screen.getByText('Datas possíveis por regra')).toBeInTheDocument();
    expect(screen.getByText('19/05/2027')).toBeInTheDocument();
    expect(screen.getByText('R$ 3.413,25')).toBeInTheDocument();
    expect(screen.getByText(/Não aplicável/)).toBeInTheDocument();
    expect(screen.queryByText(/Faltam: nada/)).not.toBeInTheDocument();
  });

  it('shows missing time as not measured instead of zero',()=>{
    render(<ResultView result={{status:'experimental',data:{native_result:{resumo:{rmi:'1600.00'},tempo:{total:{},marcos:{},vinculos:[]}}}} as Json}/>);
    expect(screen.getAllByText('Não apurado').length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByText('Sem dado').length).toBeGreaterThanOrEqual(3);
    expect(screen.queryByText('0a 0m 0d')).not.toBeInTheDocument();
  });

  it('keeps an explicit zero returned by the engine',()=>{
    render(<ResultView result={{status:'experimental',data:{native_result:{resumo:{rmi:'0.00',carencia_meses:0},tempo:{total:{anos:0,meses:0,dias:0},carencia_meses:0,marcos:{ate_ec103:0},vinculos:[]}}}} as Json}/>);
    expect(screen.getByText('0a 0m 0d')).toBeInTheDocument();
    expect(screen.getAllByText('0 meses').length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText('R$ 0,00')).toBeInTheDocument();
  });

  it('groups blocked pendencies by next action and keeps origin fields',()=>{
    const blocked:Json={status:'blocked',blockers:[
      {code:'link_unconfirmed',field:'vinculos[0]',message:'Período sem confirmação documental.'},
      {code:'source_end_after_dib',field:'cnis.links[1]',message:'Fim da origem posterior à DIB.'},
      {code:'wage_after_dib',field:'competencias[3]',message:'Remuneração posterior à DIB.'},
      {code:'wage_unconfirmed',field:'competencias[4]',message:'Salário sem confirmação documental.'},
      {code:'disability_unconfirmed',field:'segurado.deficiencia',message:'Deficiência não confirmada.'},
    ]};
    render(<ResultView result={blocked}/>);
    expect(screen.getByText('Execução bloqueada')).toBeInTheDocument();
    const linksGroup=screen.getByText('Revisar vínculos e regime').closest('details');
    expect(linksGroup).not.toBeNull();
    expect(within(linksGroup!).getAllByRole('listitem')).toHaveLength(2);
    expect(within(linksGroup!).getByText('vinculos[0]')).toBeInTheDocument();
    expect(screen.getByText('Revisar competências e remunerações')).toBeInTheDocument();
    expect(screen.getByText('Confirmar parâmetros e revisão')).toBeInTheDocument();
    expect(screen.getByText('competencias[3]')).toBeInTheDocument();
    expect(screen.queryByText(/^R\$/)).not.toBeInTheDocument();
  });
});
