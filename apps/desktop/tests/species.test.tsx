import {render,screen} from '@testing-library/react';
import {describe,expect,it} from 'vitest';
import {ResultView} from '../src/components/CaseWorkbench';
import {ESPECIES,blankCase,robotParameterIssues} from '../src/model';

function completeCase() {
  const c = blankCase();
  return {
    ...c,
    identity: {...c.identity, full_name: 'Pessoa Fictícia', birth_date: '1956-01-10'},
    sex: 'Masculino',
    disability: 'Sem deficiência declarada',
    dib: '2026-01-10',
    especie: '41',
  };
}

describe('espécies do motor', () => {
  it('oferece apenas as quatro espécies implementadas', () => {
    expect(ESPECIES).toEqual(['42', '41', '32', '31']);
  });

  it('exige espécie conhecida nos parâmetros', () => {
    expect(robotParameterIssues(completeCase())).toEqual([]);
    expect(robotParameterIssues({...completeCase(), especie: '46'})).toContain(
      'Espécie do benefício',
    );
  });

  it('mostra a espécie, a regra e os requisitos no resultado', () => {
    render(
      <ResultView
        result={{
          status: 'experimental',
          data: {
            native_result: {
              resumo: {
                rmi: '2100.00',
                salario_beneficio: '3000.00',
                competencias: 300,
                coeficiente: '70',
                carencia_meses: 300,
              },
              especie: {
                codigo: '41',
                nome: 'Aposentadoria por idade',
                regra_escolhida: 'idade-permanente',
                elegivel: true,
              },
              beneficio: {
                regras: [
                  {
                    nome: 'Regra permanente por idade',
                    requisitos: [
                      {
                        descricao: 'Idade mínima',
                        exigido: '65 anos',
                        apurado: '70a 0m',
                        cumprido: true,
                        verificado_pelo_motor: true,
                      },
                    ],
                  },
                ],
              },
              memoria: [],
              avisos: [],
            },
          },
        }}
      />,
    );
    expect(screen.getByText('Aposentadoria por idade')).toBeInTheDocument();
    expect(screen.getByText('idade-permanente')).toBeInTheDocument();
    expect(screen.getByText('Idade mínima')).toBeInTheDocument();
    expect(screen.getByText('Cumprido')).toBeInTheDocument();
  });
});
