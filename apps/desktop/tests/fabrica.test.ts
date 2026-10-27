import {describe,expect,it} from 'vitest';
import {blankCase} from '../src/model';
import {buildFabricaInput,cents,difference,localRmi} from '../src/fabrica';

function case41() {
  const c = blankCase();
  c.identity = {...c.identity, full_name: 'Pessoa Fictícia', cpf: '00000000000', birth_date: '1956-01-10'};
  c.sex = 'Masculino';
  c.disability = 'Sem deficiência declarada';
  c.dib = '2026-01-10';
  c.especie = '41';
  c.options.rule_policy = JSON.stringify({
    version: 'rgps-core-observational-v1',
    manual_link_kind: 'empregado',
    native_options: {incapacidadeOrigem: 'comum'},
  });
  c.manual_links = [
    {id: 1, employer: 'Empresa Fictícia', start_date: '1994-07-01', end_date: '2019-06-30', regime: 'RGPS', indicators: '', confirmed: true},
  ];
  c.manual_salaries = [
    {id: 1, competence: '1994-07', amount: '2000.00', indicators: '', confirmed: true},
    {id: 2, competence: '2019-06', amount: '3000.00', indicators: '', confirmed: true},
  ];
  return c;
}

describe('conferência na Fábrica de Cálculos', () => {
  it('monta o payload só com datas, sexo, espécie e salários revisados', () => {
    const built = buildFabricaInput(case41());
    expect(built.issues).toEqual([]);
    expect(built.input).not.toBeNull();
    const input = built.input!;
    expect(input).toMatchObject({
      dataApuracao: '2026-01-10',
      dataNascimento: '1956-01-10',
      sexo: 'M',
      especie: 41,
      isencaoCarencia: false,
      tipo: 'COMUM',
    });
    expect(input.periodos).toHaveLength(1);
    expect(input.periodos[0].salarios).toEqual([
      {competencia: '1994-07', valor: '2000.00'},
      {competencia: '2019-06', valor: '3000.00'},
    ]);
    const serialized = JSON.stringify(input);
    expect(serialized).not.toContain('Pessoa Fictícia');
    expect(serialized).not.toContain('00000000000');
    expect(serialized).not.toContain('Empresa Fictícia');
  });

  it('recusa vínculo ou salário não confirmado e remuneração fora do vínculo', () => {
    const unconfirmedLink = case41();
    unconfirmedLink.manual_links[0].confirmed = false;
    expect(buildFabricaInput(unconfirmedLink).input).toBeNull();

    const unconfirmedSalary = case41();
    unconfirmedSalary.manual_salaries[0].confirmed = false;
    expect(buildFabricaInput(unconfirmedSalary).input).toBeNull();

    const outside = case41();
    outside.manual_salaries.push({id: 3, competence: '2020-01', amount: '3000.00', indicators: '', confirmed: true});
    const built = buildFabricaInput(outside);
    expect(built.input).toBeNull();
    expect(built.issues.join(' ')).toContain('fora dos vínculos');
  });

  it('recusa PcD e períodos sobrepostos na conferência comum', () => {
    const pcd = case41();
    pcd.disability = 'Grave';
    expect(buildFabricaInput(pcd).input).toBeNull();
    expect(buildFabricaInput(pcd).issues.join(' ')).toContain('LC 142');

    const overlap = case41();
    overlap.manual_links.push({
      id: 2, employer: 'Outra fictícia', start_date: '2019-01-01', end_date: '2019-12-31', regime: 'RGPS', indicators: '', confirmed: true,
    });
    expect(buildFabricaInput(overlap).input).toBeNull();
    expect(buildFabricaInput(overlap).issues.join(' ')).toContain('sobrepostos');
  });

  it('extrai a RMI local corrigida e compara em centavos', () => {
    const result = {
      status: 'experimental',
      data: {
        native_result: {
          resumo: {rmi: '1600.00'},
          transicoes: {rmi_escolhida: '1700.00'},
          parametros: {correcao_ativa: true},
        },
      },
    };
    const local = localRmi(result);
    expect(local.rmi).toBe('1700.00');
    expect(local.corrected).toBe(true);
    expect(cents('1700.00')).toBe(170000n);
    const delta = difference('1600.00', '1700.00');
    expect(delta?.cents).toBe(10000n);
    expect(delta?.percent).toBe('6.3');
  });
});
