import {Badge, Notice, Panel} from './ui';

type Status = 'Implementado'|'Parcial'|'Importado, sem cálculo'|'Ausente'|'Bloqueado'|'Não copiado';
interface Row {area:string;field:string;model:string;editing:string;calculation:string;status:Status}
const rows:Row[]=[
  {area:'Entrevista',field:'Nome do cálculo',model:'Persistido no cálculo',editing:'Campo inicial',calculation:'Não altera a RMI',status:'Implementado'},
  {area:'Entrevista',field:'Benefício atual',model:'Resposta assistida (Sim/Não/Não confirmado)',editing:'Pergunta curta',calculation:'Bloqueia revisão/restabelecimento',status:'Implementado'},
  {area:'Entrevista',field:'Importação do INSS',model:'CNIS lido localmente',editing:'Aplicar ou descartar',calculation:'Fonte não entra no motor',status:'Implementado'},
  {area:'Entrevista',field:'Finalidade / processo',model:'Resposta assistida',editing:'Pergunta curta',calculation:'Registra cenário; não escolhe regra',status:'Parcial'},
  {area:'Entrevista',field:'Objetivo do cálculo',model:'Resposta assistida',editing:'Pergunta curta',calculation:'Somente novo benefício executa',status:'Parcial'},
  {area:'Entrevista',field:'Sexo cadastral',model:'Masculino/Feminino',editing:'Seleção',calculation:'Usado pelo fator e regras',status:'Implementado'},
  {area:'Entrevista',field:'Deficiência',model:'Não/Leve/Moderada/Grave',editing:'Seleção',calculation:'LC 142 com grau constante',status:'Implementado'},
  {area:'Entrevista',field:'Espécie de benefício',model:'42, 41, 32, 31, 46 e 57',editing:'Seleção',calculation:'46/57 usam tempo especial/magistério declarados; as demais bloqueiam',status:'Implementado'},
  {area:'Qualidade',field:'Filiação',model:'Importada e exibida',editing:'Somente leitura',calculation:'Classificação por vínculo',status:'Implementado'},
  {area:'Qualidade',field:'Primeiro pagamento',model:'Derivado do primeiro lançamento',editing:'Conferência',calculation:'Primeira competência com salário',status:'Implementado'},
  {area:'Qualidade',field:'Período de graça / prorrogações',model:'12 meses de graça e prorrogação até 24',editing:'Não se aplica',calculation:'Aplicado por vínculo',status:'Implementado'},
  {area:'Qualidade',field:'Contribuições desconsideradas',model:'Livro de revisão e flags da fonte',editing:'Marcar por registro',calculation:'Exclui da carência',status:'Implementado'},
  {area:'Qualidade',field:'Cálculo da qualidade por vínculo',model:'Relatório próprio com 12 meses de carência',editing:'Conferência',calculation:'Apuração própria',status:'Implementado'},
  {area:'Qualidade',field:'Fim de doença / reclusão',model:'Não armazenado',editing:'Não se aplica',calculation:'Não reproduzido',status:'Ausente'},
  {area:'Qualidade',field:'Desemprego / forças armadas',model:'Não armazenado',editing:'Não se aplica',calculation:'Não reproduzido',status:'Ausente'},
  {area:'Regras',field:'Lei 8.213, idade mínima',model:'65/60 anos por sexo',editing:'Declarar sexo',calculation:'Regra de pontos',status:'Implementado'},
  {area:'Regras',field:'Lei 8.213, tempo mínimo',model:'35/30 anos por sexo',editing:'Não se aplica',calculation:'Regra de pontos',status:'Implementado'},
  {area:'Regras',field:'EC 103, regra de transição',model:'Pontos, idade progressiva, pedágio',editing:'Seleção ou automática',calculation:'Todas as regras',status:'Implementado'},
  {area:'Regras',field:'Lei 9.876, fator previdenciário',model:'Alíquota 0,31 + 5 anos mulher',editing:'Declarar sexo',calculation:'Sem piso artificial',status:'Implementado'},
  {area:'Regras',field:'Lei 9.876, descarte de 20%',model:'Opção da política',editing:'Declarar true/false',calculation:'Aplicado como opção',status:'Implementado'},
  {area:'Regras',field:'Lei 9.876, fator / concomitância',model:'Sem cálculo validado',editing:'Não se aplica',calculation:'Bloqueado com explicação',status:'Bloqueado'},
  {area:'Regras',field:'Leis anteriores / direito adquirido',model:'Sem motor',editing:'Não se aplica',calculation:'DIB anterior a reforma bloqueada',status:'Ausente'},
  {area:'Regras',field:'Data de correção',model:'Referência da política',editing:'Competência declarada',calculation:'INPC quando suportado',status:'Implementado'},
  {area:'Relatórios',field:'Oito tipos próprios',model:'Renderer Rust',editing:'Seleção e prévia',calculation:'Recalcula a hipótese',status:'Implementado'},
  {area:'Relatórios',field:'Relatório unificado',model:'Detalhado com anexo técnico',editing:'Prévia/exportação',calculation:'Recalcula a hipótese',status:'Implementado'},
  {area:'Relatórios',field:'Seleção em lote',model:'Comando nativo por pasta',editing:'Marcar vários tipos',calculation:'Um PDF por tipo; não sobrescreve',status:'Implementado'},
  {area:'Relatórios',field:'Títulos editáveis',model:'Título editável e persistido',editing:'Não se aplica',calculation:'Muda só o documento',status:'Implementado'},
  {area:'Relatórios',field:'Opções de apresentação',model:'Identificação pessoal, anexo técnico, conferência de qualidade e premissas de planejamento',editing:'Somente essas opções',calculation:'Não alteram o cálculo',status:'Parcial'},
  {area:'Relatórios',field:'Conclusão e pendências decisivas',model:'Seção de conclusão',editing:'Conferência antes de gerar',calculation:'Lista o que pode mudar a conclusão',status:'Implementado'},
  {area:'Relatórios',field:'Memória completa',model:'Memória e correção do motor',editing:'Conferência',calculation:'Todas as etapas retornadas',status:'Implementado'},
  {area:'Relatórios',field:'Qualidade / retorno preenchidos',model:'Motor próprio com 12 meses de carência',editing:'Não se aplica',calculation:'Relatório de qualidade por vínculo',status:'Implementado'},
  {area:'Relatórios',field:'Mesclagem de PDFs',model:'merge_pdfs combina N PDFs',editing:'Seleção de arquivos',calculation:'Preserva ordem das páginas',status:'Implementado'},
  {area:'Relatórios',field:'Paginação n/N',model:'Rodapé Página X/Y',editing:'Automático',calculation:'Duas passadas de renderização',status:'Implementado'},
  {area:'Planejamento',field:'Projeção de DER',model:'project_rgps expõe projetar',editing:'Dados confirmados',calculation:'Simula até primeira regra elegível',status:'Implementado'},
  {area:'Relatórios',field:'Análise da Situação Atual',model:'pdf_analysis com requisitos e financeiro',editing:'Seleção de tipo',calculation:'A PAGAR vs A RECEBER + ROI',status:'Implementado'},
];
const tone=(s:Status)=>s==='Implementado'?'good':s==='Parcial'||s==='Importado, sem cálculo'?'review':'neutral';
export function CoverageMap(){
  const counts=rows.reduce<Record<string,number>>((acc,row)=>{acc[row.status]=(acc[row.status]??0)+1;return acc},{});
  return <>
    <div className="result-strip">{(['Implementado','Parcial','Importado, sem cálculo','Bloqueado','Ausente','Não copiado']as Status[]).map(status=><div key={status}><span>{status}</span><strong>{counts[status]??0}</strong></div>)}</div>
    <Panel title="Mapa de cobertura do motor">
      <Notice>Este mapa descreve o que o motor local implementa hoje. Não é uma certidão de equivalência com o produto de referência.</Notice>
      <table><thead><tr><th>Área</th><th>Campo</th><th>Modelo</th><th>Edição</th><th>Cálculo</th><th>Situação</th></tr></thead><tbody>
        {rows.map((row,i)=><tr key={i}><td>{row.area}</td><td>{row.field}</td><td>{row.model}</td><td>{row.editing}</td><td>{row.calculation}</td><td><Badge tone={tone(row.status)}>{row.status}</Badge></td></tr>)}
      </tbody></table>
    </Panel>
  </>;
}