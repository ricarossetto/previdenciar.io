export type OrigemTexto = 'nativo' | 'ocr';
export type TipoLeitura =
  | 'cnis'
  | 'laudo_medico'
  | 'processo_administrativo'
  | 'ppp'
  | 'prova_rural'
  | 'grupo_familiar'
  | 'carta_de_concessao'
  | 'documentos_da_pensao'
  | 'documento_generico';
export type StatusLeitura = 'em_revisao' | 'confirmada';
export type ConfiancaCampo = 'alta' | 'media' | 'baixa';
export type ProvenienciaFato = 'comprovado' | 'alegado' | 'inferido';
export type MotivoPendencia = 'ausente_no_documento' | 'ilegivel' | 'conflitante' | 'nao_informado';
export type SeveridadeAlerta = 'critico' | 'atencao' | 'info';

export interface Documento {
  id: string;
  nome_arquivo: string;
  mime: string;
  paginas: number;
  origem_texto: OrigemTexto;
  ocr_confianca: number | null;
  hash_sha256: string;
  criado_em: string;
}

export interface OrigemCampo {
  documento_id: string;
  pagina: number;
  trecho: string;
}

export interface CampoExtraido {
  chave: string;
  valor: string | number | boolean | null;
  confianca: ConfiancaCampo;
  origem: OrigemCampo;
}

export interface Pendencia {
  campo: string;
  motivo: MotivoPendencia;
  pergunta: string;
}

export interface Leitura {
  id: string;
  documento_id: string;
  tipo: TipoLeitura;
  campos: CampoExtraido[];
  paginas_lidas: number;
  pendencias: Pendencia[];
  status: StatusLeitura;
  texto_extraido: string | null;
}

export interface Fato {
  id: string;
  texto: string;
  proveniencia: ProvenienciaFato;
  origem: OrigemCampo;
  alegado_por: string | null;
  confirmado: boolean;
  usado_em: number[];
}

export interface Alerta {
  codigo: string;
  severidade: SeveridadeAlerta;
  mensagem: string;
}
