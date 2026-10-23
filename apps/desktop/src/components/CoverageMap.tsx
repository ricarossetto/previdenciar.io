import {Badge, Notice, Panel} from './ui';

type Status = 'Implementado'|'Parcial'|'Importado, sem cálculo'|'Ausente'|'Bloqueado'|'Não copiado';
interface Row {area:string;field:string;model:string;editing:string;calculation:string;status:Status}
const rows:Row[]=[
  {area:'Entrevista',field:'Nome do cálculo',model:'Persistido no cálculo',editing:'Campo inicial',calculation:'Não altera a RMI',status:'Implementado'},
  {area:'Entrevista',field:'Benefício atual',model:'Resposta assistida (Sim/Não/Não confirmado)',editing:'Pergunta curta',calculation:'Bloqueia revisão/restabelecimento',status:'Implementado'},
  {area:'Entrevista',field:'Importação do INSS',model:'CNIS lido localmente',editing:'Aplicar ou descartar',calculation:'Fonte não entra no motor',status:'Implementado'},
  {area:'Entrevista',field:'Finalidade / processo',model:'Resposta assistida',editing:'Pergunta curta',calculation:'Registra cenário; não escolhe regra',status:'Parcial'},
  {area:'Entrevista',field:'Objetivo do cálculo',model:'Resposta assistida',editing:'Pergunta curta',calculation:'Somente novo benefício executa',status:'Parcial'},
  {area:'Entrevista',field:'Nascimento / inscrição',model:'Nascimento e NIT importados',editing:'Nascimento editável; NIT só exibido',calculation:'Nascimento e DIB alimentam o motor',status:'Parcial'},
  {area:'Entrevista',field:'DIB / sexo',model:'Campos do cálculo',editing:'Confirmados pelo operador',calculation:'Usados pelo motor',status:'Implementado'},
  {area:'Entrevista',field:'Espécie do benefício',model:'Espécie 42 no contrato; demais códigos bloqueiam',editing:'Seleção no cenário',calculation:'Motor atende a 42 e bloqueia o resto',status:'Parcial'},
  {area:'Entrevista',field:'Deficiência',model:'Grau declarado',editing:'Leve/Moderada/Grave/Sem/A apurar',calculation:'LC 142 experimental; sem graus mistos',status:'Parcial'},
  {area:'Vínculo',field:'Datas de início e fim',model:'Fonte + ajuste com histórico',editing:'Editor por vínculo',calculation:'Linha manual confirmada',status:'Implementado'},
  {area:'Vínculo',field:'Tipo de vínculo',model:'Categoria global da política',editing:'Não é por vínculo',calculation:'Valida categoria única',status:'Parcial'},
  {area:'Vínculo',field:'Empresa / empregador',model:'Origem e ajuste',editing:'Editor por vínculo',calculation:'Não altera a RMI',status:'Implementado'},
  {area:'Vínculo',field:'Função / cargo',model:'Não armazenado',editing:'—',calculation:'Não usado',status:'Ausente'},
  {area:'Vínculo',field:'Grau/fator por período',model:'Sem campo por período',editing:'—',calculation:'Motor exige fator 1,0 e bloqueia conversão',status:'Bloqueado'},
  {area:'Vínculo',field:'Graus mistos',model:'Sem estrutura de graus por período',editing:'—',calculation:'Não implementado',status:'Ausente'},
  {area:'Vínculo',field:'Indicadores',model:'Preservados da fonte',editing:'Conferência documental',calculation:'Indicador pendente bloqueia',status:'Implementado'},
  {area:'Vínculo',field:'Origem / situação',model:'Página, linha, histórico e exclusão',editing:'Origem preservada',calculation:'Ajuste excluído não entra',status:'Parcial'},
  {area:'Vínculo',field:'Carência por vínculo',model:'Sem campo por vínculo',editing:'—',calculation:'Motor apura carência total',status:'Ausente'},
  {area:'Vínculo',field:'Recolhimento simplificado 11%/5%',model:'Não armazenado',editing:'—',calculation:'Não implementado',status:'Ausente'},
  {area:'Vínculo',field:'Comprovação por registro',model:'Referência documental global',editing:'Política do cálculo',calculation:'Evidência única para as linhas',status:'Parcial'},
  {area:'Vínculo',field:'Exclusão do vínculo',model:'Overlay com justificativa',editing:'Excluir sem apagar origem',calculation:'Linha excluída não entra',status:'Implementado'},
  {area:'Qualidade',field:'Filiação',model:'Importada e exibida',editing:'Somente leitura',calculation:'Qualidade apurada por vínculo (art. 15)',status:'Parcial'},
  {area:'Qualidade',field:'Primeiro pagamento',model:'Derivado do primeiro lançamento',editing:'Conferência por vínculo',calculation:'Informativo; não calcula graça',status:'Importado, sem cálculo'},
  {area:'Qualidade',field:'Período de graça / prorrogações',model:'12 meses de graça e prorrogação até 24 com mais de 120 contribuições',editing:'Relatório de qualidade',calculation:'Motor apura (art. 15, II e § 1º)',status:'Parcial'},
  {area:'Qualidade',field:'Contribuições desconsideradas',model:'Livro de revisão e flags da fonte',editing:'Conferência e exclusão',calculation:'Projeção aplicada ao PBC',status:'Implementado'},
  {area:'Qualidade',field:'Cálculo da qualidade por vínculo',model:'Relatório próprio com 12 meses de graça e prorrogação por 120 contribuições (art. 15, II e § 1º, Lei 8.213/91)',editing:'Conferência por vínculo',calculation:'Cálculo próprio, carência apenas comparada',status:'Parcial'},
  {area:'Qualidade',field:'Fim de doença / reclusão',model:'Não armazenado',editing:'—',calculation:'Não implementado',status:'Ausente'},
  {area:'Qualidade',field:'Desemprego / forças armadas',model:'Não armazenado',editing:'—',calculation:'Não implementado',status:'Ausente'},
  {area:'Remuneração',field:'Competência',model:'Lançamento com origem',editing:'Livro por competência',calculation:'Entra no PBC após revisão',status:'Implementado'},
  {area:'Remuneração',field:'Data de pagamento',model:'Não armazenada',editing:'—',calculation:'Não usada',status:'Ausente'},
  {area:'Remuneração',field:'Empresa / valor principal',model:'Vínculo por sequência',editing:'Editor de remunerações',calculation:'Valor confirmado entra',status:'Parcial'},
  {area:'Remuneração',field:'Cinco valores secundários',model:'Lançamentos múltiplos',editing:'Consolidação documental',calculation:'Concomitância exige merge',status:'Parcial'},
  {area:'Remuneração',field:'Indicadores',model:'Preservados',editing:'Conferência documental',calculation:'Indicador pendente bloqueia',status:'Implementado'},
  {area:'Remuneração',field:'Origem',model:'Página, linha e índice',editing:'Somente leitura',calculation:'Rastreabilidade',status:'Implementado'},
  {area:'Remuneração',field:'Concomitância',model:'Livro com merge por competência',editing:'Consolidar lançamentos',calculation:'Duplicidade não consolidada bloqueia',status:'Parcial'},
  {area:'Planejamento',field:'Salário futuro / frequência',model:'Sem campos',editing:'—',calculation:'Não implementado',status:'Ausente'},
  {area:'Planejamento',field:'Contribuição / alíquota',model:'Sem campos',editing:'—',calculation:'Não implementado',status:'Ausente'},
  {area:'Planejamento',field:'Investimento / retorno / IRRF',model:'Sem campos',editing:'—',calculation:'Não implementado',status:'Ausente'},
  {area:'Planejamento',field:'Crescimento anual da RMI',model:'Sem campo',editing:'—',calculation:'Não implementado',status:'Ausente'},
  {area:'Planejamento',field:'Expectativa de vida',model:'Declarada na política',editing:'Campo declarado',calculation:'Caminho de fator bloqueado',status:'Bloqueado'},
  {area:'Regras',field:'EC 103 — descarte',model:'Opção da política',editing:'Declarar true/false',calculation:'Aplicado como opção',status:'Implementado'},
  {area:'Regras',field:'EC 103 — acréscimo de 25%',model:'Sem campo',editing:'—',calculation:'Não implementado',status:'Ausente'},
  {area:'Regras',field:'EC 103 — coeficiente manual',model:'Sem campo',editing:'—',calculation:'Não implementado',status:'Ausente'},
  {area:'Regras',field:'Lei 9.876 — descarte de 20%',model:'Opção da política',editing:'Declarar true/false',calculation:'Aplicado como opção',status:'Implementado'},
  {area:'Regras',field:'Lei 9.876 — fator / concomitância',model:'Sem cálculo validado',editing:'—',calculation:'Bloqueado com explicação',status:'Bloqueado'},
  {area:'Regras',field:'Leis anteriores / direito adquirido',model:'Sem motor',editing:'—',calculation:'DIB anterior à reforma bloqueada',status:'Ausente'},
  {area:'Regras',field:'Data de correção',model:'Referência da política',editing:'Competência declarada',calculation:'INPC quando suportado',status:'Implementado'},
  {area:'Relatórios',field:'Oito tipos próprios',model:'Renderer Rust',editing:'Seleção e prévia',calculation:'Recalcula a hipótese',status:'Implementado'},
  {area:'Relatórios',field:'Relatório unificado',model:'Detalhado com anexo técnico',editing:'Prévia/exportação',calculation:'Recalcula a hipótese',status:'Implementado'},
  {area:'Relatórios',field:'Seleção em lote',model:'Comando nativo por pasta',editing:'Marcar vários tipos',calculation:'Um PDF por tipo; sem sobrescrever',status:'Implementado'},
  {area:'Relatórios',field:'Títulos editáveis',model:'Título fixo por tipo',editing:'—',calculation:'—',status:'Implementado'},
  {area:'Relatórios',field:'Opções de apresentação',model:'Identificação pessoal, anexo técnico, conferência de qualidade e premissas de planejamento',editing:'Somente essas opções',calculation:'Não alteram o cálculo',status:'Parcial'},
  {area:'Relatórios',field:'Conclusão e pendências decisivas',model:'Seção de conclusão',editing:'Conferência antes de gerar',calculation:'Lista o que pode mudar a conclusão',status:'Implementado'},
  {area:'Relatórios',field:'Memória completa',model:'Memória e correção do motor',editing:'Conferência',calculation:'Todas as etapas retornadas',status:'Implementado'},
  {area:'Relatórios',field:'Qualidade / retorno preenchidos',model:'Sem motor validado',editing:'—',calculation:'Não inventados',status:'Não copiado'},
];
const tone=(status:Status)=>status==='Implementado'?'good':status==='Parcial'||status==='Importado, sem cálculo'?'review':'neutral';
export function CoverageMap(){
  const counts=rows.reduce<Record<string,number>>((acc,row)=>{acc[row.status]=(acc[row.status]??0)+1;return acc},{});
  return <>
    <div className="result-strip">{(['Implementado','Parcial','Importado, sem cálculo','Bloqueado','Ausente','Não copiado']as Status[]).map(status=><div key={status}><span>{status}</span><strong>{counts[status]??0}</strong></div>)}</div>
    <Panel title="Mapa dos campos observados" subtitle="Comparação funcional com a interface auditada em 21/10/2026 — não é equivalência nem prova de fórmula.">
      <div className="table-wrap"><table className="data-table"><thead><tr><th>Área</th><th>Campo observado</th><th>Modelo / persistência</th><th>Edição e conferência</th><th>Cálculo</th><th>Situação</th></tr></thead><tbody>{rows.map((row,index)=><tr key={row.area+row.field+index}><td>{row.area}</td><td><strong>{row.field}</strong></td><td>{row.model}</td><td>{row.editing}</td><td>{row.calculation}</td><td><Badge tone={tone(row.status)}>{row.status}</Badge></td></tr>)}</tbody></table></div>
      <Notice>“Implementado” significa contrato, persistência, edição e teste no escopo desta aplicação. “Bloqueado” significa que o motor recusa a operação em vez de ignorá-la. “Ausente” não é zero nem estimativa: é uma lacuna declarada.</Notice>
    </Panel>
  </>;
}
