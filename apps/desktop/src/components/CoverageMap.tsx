import {Badge, Notice, Panel} from './ui';

type Status = 'Implementado'|'Parcial'|'Importado, sem calculo'|'Ausente'|'Bloqueado'|'Nao copiado';
interface Row {area:string;field:string;model:string;editing:string;calculation:string;status:Status}
const rows:Row[]=[
  {area:'Entrevista',field:'Nome do calculo',model:'Persistido no calculo',editing:'Campo inicial',calculation:'Nao altera a RMI',status:'Implementado'},
  {area:'Entrevista',field:'Beneficio atual',model:'Resposta assistida (Sim/Nao/Nao confirmado)',editing:'Pergunta curta',calculation:'Bloqueia revisao/restabelecimento',status:'Implementado'},
  {area:'Entrevista',field:'Importacao do INSS',model:'CNIS lido localmente',editing:'Aplicar ou descartar',calculation:'Fonte nao entra no motor',status:'Implementado'},
  {area:'Entrevista',field:'Finalidade / processo',model:'Resposta assistida',editing:'Pergunta curta',calculation:'Registra cenario; nao escolhe regra',status:'Parcial'},
  {area:'Entrevista',field:'Objetivo do calculo',model:'Resposta assistida',editing:'Pergunta curta',calculation:'Somente novo beneficio executa',status:'Parcial'},
  {area:'Entrevista',field:'Sexo cadastral',model:'Masculino/Feminino',editing:'Selecao',calculation:'Usado pelo fator e regras',status:'Implementado'},
  {area:'Entrevista',field:'Deficiencia',model:'Nao/Leve/Moderada/Grave',editing:'Selecao',calculation:'LC 142 com grau constante',status:'Implementado'},
  {area:'Entrevista',field:'Especie de beneficio',model:'Texto livre',editing:'Campo',calculation:'Apenas 42 tem motor; demais bloqueiam',status:'Parcial'},
  {area:'Qualidade',field:'Filiacao',model:'Importada e exibida',editing:'Somente leitura',calculation:'Classificacao por vinculo',status:'Implementado'},
  {area:'Qualidade',field:'Primeiro pagamento',model:'Derivado do primeiro lancamento',editing:'Conferencia',calculation:'Primeira competencia com salario',status:'Implementado'},
  {area:'Qualidade',field:'Periodo de graca / prorrogacoes',model:'12 meses de graca e prorrogacao ate 24',editing:'Nao se aplica',calculation:'Aplicado por vinculo',status:'Implementado'},
  {area:'Qualidade',field:'Contribuicoes desconsideradas',model:'Livro de revisao e flags da fonte',editing:'Marcar por registro',calculation:'Exclui da carencia',status:'Implementado'},
  {area:'Qualidade',field:'Calculo da qualidade por vinculo',model:'Relatorio proprio com 12 meses de carencia',editing:'Conferencia',calculation:'Apuracao propria',status:'Implementado'},
  {area:'Qualidade',field:'Fim de doenca / reclusao',model:'Nao armazenado',editing:'Nao se aplica',calculation:'Nao reproduzido',status:'Ausente'},
  {area:'Qualidade',field:'Desemprego / forcas armadas',model:'Nao armazenado',editing:'Nao se aplica',calculation:'Nao reproduzido',status:'Ausente'},
  {area:'Regras',field:'Lei 8.213, idade minima',model:'65/60 anos por sexo',editing:'Declarar sexo',calculation:'Regra de pontos',status:'Implementado'},
  {area:'Regras',field:'Lei 8.213, tempo minimo',model:'35/30 anos por sexo',editing:'Nao se aplica',calculation:'Regra de pontos',status:'Implementado'},
  {area:'Regras',field:'EC 103, regra de transicao',model:'Pontos, idade progressiva, pedagio',editing:'Selecao ou automatica',calculation:'Todas as regras',status:'Implementado'},
  {area:'Regras',field:'Lei 9.876, fator previdenciario',model:'Aliquota 0,31 + 5 anos mulher',editing:'Declarar sexo',calculation:'Sem piso artificial',status:'Implementado'},
  {area:'Regras',field:'Lei 9.876, descarte de 20%',model:'Opcao da politica',editing:'Declarar true/false',calculation:'Aplicado como opcao',status:'Implementado'},
  {area:'Regras',field:'Lei 9.876, fator / concomitancia',model:'Sem calculo validado',editing:'Nao se aplica',calculation:'Bloqueado com explicacao',status:'Bloqueado'},
  {area:'Regras',field:'Leis anteriores / direito adquirido',model:'Sem motor',editing:'Nao se aplica',calculation:'DIB anterior a reforma bloqueada',status:'Ausente'},
  {area:'Regras',field:'Data de correcao',model:'Referencia da politica',editing:'Competencia declarada',calculation:'INPC quando suportado',status:'Implementado'},
  {area:'Relatorios',field:'Oito tipos proprios',model:'Renderer Rust',editing:'Selecao e previa',calculation:'Recalcula a hipotese',status:'Implementado'},
  {area:'Relatorios',field:'Relatorio unificado',model:'Detalhado com anexo tecnico',editing:'Previa/exportacao',calculation:'Recalcula a hipotese',status:'Implementado'},
  {area:'Relatorios',field:'Selecao em lote',model:'Comando nativo por pasta',editing:'Marcar varios tipos',calculation:'Um PDF por tipo; nao sobrescreve',status:'Implementado'},
  {area:'Relatorios',field:'Titulos editaveis',model:'Titulo editavel e persistido',editing:'Nao se aplica',calculation:'Muda so o documento',status:'Implementado'},
  {area:'Relatorios',field:'Opcoes de apresentacao',model:'Identificacao pessoal, anexo tecnico, conferencia de qualidade e premissas de planejamento',editing:'Somente essas opcoes',calculation:'Nao alteram o calculo',status:'Parcial'},
  {area:'Relatorios',field:'Conclusao e pendencias decisivas',model:'Secao de conclusao',editing:'Conferencia antes de gerar',calculation:'Lista o que pode mudar a conclusao',status:'Implementado'},
  {area:'Relatorios',field:'Memoria completa',model:'Memoria e correcao do motor',editing:'Conferencia',calculation:'Todas as etapas retornadas',status:'Implementado'},
  {area:'Relatorios',field:'Qualidade / retorno preenchidos',model:'Motor proprio com 12 meses de carencia',editing:'Nao se aplica',calculation:'Relatorio de qualidade por vinculo',status:'Implementado'},
  {area:'Relatorios',field:'Mesclagem de PDFs',model:'merge_pdfs combina N PDFs',editing:'Selecao de arquivos',calculation:'Preserva ordem das paginas',status:'Implementado'},
  {area:'Relatorios',field:'Paginacao n/N',model:'Rodape Pagina X/Y',editing:'Automatico',calculation:'Duas passadas de renderizacao',status:'Implementado'},
  {area:'Planejamento',field:'Projecao de DER',model:'project_rgps expoe projetar',editing:'Dados confirmados',calculation:'Simula ate primeira regra elegivel',status:'Implementado'},
  {area:'Relatorios',field:'Analise da Situacao Atual',model:'pdf_analysis com requisitos e financeiro',editing:'Selecao de tipo',calculation:'A PAGAR vs A RECEBER + ROI',status:'Implementado'},
];
const tone=(s:Status)=>s==='Implementado'?'good':s==='Parcial'||s==='Importado, sem calculo'?'review':'neutral';
export function CoverageMap(){
  const counts=rows.reduce<Record<string,number>>((acc,row)=>{acc[row.status]=(acc[row.status]??0)+1;return acc},{});
  return <>
    <div className="result-strip">{(['Implementado','Parcial','Importado, sem calculo','Bloqueado','Ausente','Nao copiado']as Status[]).map(status=><div key={status}><span>{status}</span><strong>{counts[status]??0}</strong></div>)}</div>
    <Panel title="Mapa de cobertura do motor">
      <Notice>Este mapa descreve o que o motor local implementa hoje. Nao e uma certidao de equivalencia com o produto de referencia.</Notice>
      <table><thead><tr><th>Area</th><th>Campo</th><th>Modelo</th><th>Edicao</th><th>Calculo</th><th>Status</th></tr></thead><tbody>
        {rows.map((row,i)=><tr key={i}><td>{row.area}</td><td>{row.field}</td><td>{row.model}</td><td>{row.editing}</td><td>{row.calculation}</td><td><Badge tone={tone(row.status)}>{row.status}</Badge></td></tr>)}
      </tbody></table>
    </Panel>
  </>;
}