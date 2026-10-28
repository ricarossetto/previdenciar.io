import {render,screen} from '@testing-library/react';
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
    expect(screen.getByText('R$ 1.600,00')).toBeInTheDocument();
  });
});
