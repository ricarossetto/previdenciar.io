use crate::{ReportKind, ReportOptions};
use previus_cnis::{AmountKind, BenefitStatus, SourceRef};
use previus_workspace::{Calculation, Workspace};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
const MAX_ROWS: usize = 10_000;
const MAX_FIELD: usize = 1_000;
#[derive(Debug)] pub(crate) struct Report { pub title: String, pub status: String, pub office: String, pub footer: String, pub personal: bool, pub identification: [String; 3], pub sections: Vec<Section> }
#[derive(Debug)] pub(crate) struct Section { pub title: String, pub paragraphs: Vec<String>, pub tables: Vec<Table>, pub new_page: bool }
#[derive(Debug)] pub(crate) struct Table { pub caption: String, pub headers: Vec<String>, pub weights: Vec<usize>, pub rows: Vec<Vec<String>> }
fn section(title: &str, new_page: bool) -> Section { Section { title: title.into(), paragraphs: vec![], tables: vec![], new_page } }
fn table(caption: &str, headers: &[&str], weights: &[usize], rows: Vec<Vec<String>>) -> Table { Table { caption: caption.into(), headers: headers.iter().map(|s|s.to_string()).collect(), weights: weights.to_vec(), rows } }
fn field(value: &str) -> Result<String,String> {
    if value.chars().count() > MAX_FIELD { return Err("Campo de relatório excede 1.000 caracteres; nenhum texto foi cortado.".into()); }
    if value.chars().any(|c| c.is_control() && !c.is_whitespace()) { return Err("Texto de relatório contém caracteres de controle não suportados.".into()); }
    Ok(value.split_whitespace().collect::<Vec<_>>().join(" "))
}
fn optional_field(value: Option<&str>) -> Result<String,String> { value.filter(|s|!s.trim().is_empty()).map(field).unwrap_or_else(||Ok("Não informado".into())) }
fn opt_bool(value: Option<bool>) -> &'static str { match value { Some(true)=>"Sim", Some(false)=>"Não", None=>"Não respondido" } }
fn bool_text(value: bool) -> String { opt_bool(Some(value)).into() }
fn enum_text(value: Option<&str>, allowed: &[&str]) -> String {
    match value { Some(v) if allowed.contains(&v)=>v.to_string(), None=>"Não informado".into(), _=>"Valor fora do contrato / pendente".into() }
}
fn iso_valid(value: &str) -> bool {
    if value.len()!=10 || !value.is_ascii() || &value[4..5]!="-" || &value[7..8]!="-" { return false; }
    let y=value[..4].parse::<u32>().ok(); let m=value[5..7].parse::<u32>().ok(); let d=value[8..].parse::<u32>().ok();
    if let (Some(y),Some(m),Some(d))=(y,m,d) { let max=match m { 1|3|5|7|8|10|12=>31,4|6|9|11=>30,2=>if y%4==0&&(y%100!=0||y%400==0){29}else{28},_=>0 }; y>=1 && d>=1 && d<=max } else { false }
}
fn date(value: Option<&str>) -> String {
    match value { Some(v) if iso_valid(v)=>format!("{}/{}/{}", &v[8..],&v[5..7],&v[..4]), _=>"Não informada / inválida".into() }
}
fn month(value: &str) -> Result<String,String> {
    if value.len()==7 && value.is_ascii() && &value[4..5]=="-" && iso_valid(&format!("{value}-01")) { Ok(format!("{}/{}",&value[5..],&value[..4])) }
    else { Err("Competência de relatório inválida; esperado AAAA-MM.".into()) }
}
fn source(source: &SourceRef) -> String { format!("p. {} • linha {}",source.page,source.line) }
fn cents(value: i128) -> String {
    let s=value.unsigned_abs().to_string(); let s=if s.len()<3 {format!("{:0>3}",s)}else{s};
    let (integer,fraction)=s.split_at(s.len()-2); let mut grouped=String::new();
    for (i,c) in integer.chars().enumerate(){ if i>0 && (integer.len()-i)%3==0 { grouped.push('.'); } grouped.push(c); }
    format!("{}{grouped},{fraction}",if value<0{"-"}else{""})
}
fn decimal(value: &str, money: bool) -> Result<String,String> {
    if value.is_empty() || value.len()>64 {return Err("Valor numérico de relatório inválido.".into());}
    let v=value.strip_prefix('-').unwrap_or(value);
    let parts:Vec<_>=v.split('.').collect();
    if parts.len()>2 || parts[0].is_empty() || parts.iter().any(|p|!p.chars().all(|c|c.is_ascii_digit())) || parts.get(1).is_some_and(|p|p.is_empty()) {return Err("Valor numérico de relatório inválido.".into());}
    if money {
        let frac=parts.get(1).copied().unwrap_or("");
        if frac.len()>2 {return Err("Valor monetário não pode ser arredondado silenciosamente no relatório.".into());}
        let whole=parts[0].parse::<i128>().map_err(|_|"Valor numérico fora do limite.".to_string())?;
        let fraction=format!("{frac:0<2}").parse::<i128>().map_err(|_|"Fração monetária inválida.".to_string())?;
        let sum=whole.checked_mul(100).and_then(|w|w.checked_add(fraction)).ok_or("Valor monetário fora do limite.")?;
        Ok(cents(if value.starts_with('-'){-sum}else{sum}))
    } else { Ok(value.replace('.',",")) }
}
fn numeric(value: Option<&Value>, money: bool) -> Result<String,String> {
    match value {
        None|Some(Value::Null)=>Ok("Não produzido".into()),
        Some(Value::String(s))=>decimal(s,money),
        Some(Value::Number(n))=>decimal(&n.to_string(),money),
        _=>Err("Tipo de campo numérico do resultado inválido.".into()),
    }
}
fn manual_amount(value: &str) -> Result<String,String> {
    if value.trim().is_empty(){return Ok("Não informado".into());}
    let normalized=if value.contains(','){value.replace('.',"").replace(',',".")}else{value.to_string()};
    decimal(&normalized,true)
}
fn indicators(values: &[String], personal: bool) -> Result<String,String> {
    const KNOWN: [&str;14]=["PRPPS","RPPS","PREC-MENOR-MIN","IREM-INDPEND","IREC-INDPEND","PREM-EMPR","PREM-FVIN","PEXT","AVRC-DEF","PPSC","PSC-MEN-SM-EC103","IREM-ACD","PADM-EMPR","IEAN"];
    if personal { return if values.is_empty(){Ok("Nenhum informado".into())}else{field(&values.join(", "))}; }
    let known: BTreeSet<_>=values.iter().filter(|s|KNOWN.contains(&s.as_str())).map(String::as_str).collect();
    let hidden=values.iter().filter(|s|!KNOWN.contains(&s.as_str())).count();
    let mut text=known.into_iter().collect::<Vec<_>>().join(", ");
    if hidden>0 {if !text.is_empty(){text.push_str("; ");}text.push_str(&format!("{hidden} código(s) não catalogado(s) oculto(s)"));}
    if text.is_empty(){text="Nenhum informado".into();} Ok(text)
}
fn json_budget(value: &Value) -> Result<(),String> {
    let mut stack=vec![(value,0)]; let mut nodes=0;
    while let Some((v,depth))=stack.pop(){nodes+=1;if nodes>100_000||depth>24{return Err("Resultado excede limites estruturais do relatório.".into());}
        match v { Value::Array(a)=>{if a.len()>MAX_ROWS{return Err("Lista excede 10.000 registros.".into());}stack.extend(a.iter().map(|v|(v,depth+1)));},
            Value::Object(o)=>{if o.len()>1024{return Err("Objeto de resultado excede limite.".into());}stack.extend(o.values().map(|v|(v,depth+1)));},
            Value::String(s) if s.len()>64*1024=>return Err("Campo de resultado excede limite estrutural.".into()), _=>() }
    } Ok(())
}
fn selected(options: ReportOptions, kinds: &[ReportKind]) -> bool { options.kind==ReportKind::Detailed || kinds.contains(&options.kind) }
fn summary(case:&Calculation, options:ReportOptions, native:Option<&Value>) -> Result<Section,String> {
    let mut s=section("Síntese, situação e alcance",false);
    s.paragraphs.push("Reconstrução documental nativa em Rust. Não é reprodução idêntica do relatório proprietário do Previus; nenhum código WLanguage ou fórmula desconhecida foi presumido.".into());
    s.paragraphs.push("Legenda: FONTE = dados importados, sem revisão automática; DECLARADO = informado pelo operador; PENDENTE = exige conferência; EXPERIMENTAL = cálculo computacional, sem certificação jurídica.".into());
    s.paragraphs.push(if options.include_personal {"Relatório confidencial com identificação autorizada; revisar antes de compartilhar."}else{"Identificação direta e textos livres ocultos. Datas, valores e vínculos técnicos ainda permitem correlação: este documento não é garantia de anonimização nem de ausência de vazamento."}.into());
    let mut rows=vec![vec!["DIB / referência declarada".into(),date(Some(&case.dib))],vec!["Natureza da referência".into(),"Hipótese do cenário; não data concedida pelo INSS".into()],
        vec!["Sexo declarado".into(),enum_text(case.sex.as_deref(),&["masculino","feminino","Masculino","Feminino"])],
        vec!["Deficiência declarada".into(),enum_text(case.disability.as_deref(),&["none","Não possui deficiência","Não","nenhuma","leve","moderada","grave"])],
        vec!["Nascimento informado".into(),date(case.identity.birth_date.as_deref())]];
    if let Some(cnis)=&case.cnis {
        let months:BTreeSet<_>=cnis.salary_entries.iter().map(|v|v.competence.as_str()).collect();
        let multi=months.iter().filter(|m|cnis.salary_entries.iter().filter(|e|e.competence.as_str()==**m).count()>1).count();
        rows.extend([vec!["Páginas extraídas / páginas na fonte".into(),format!("{} / {}",cnis.metadata.parsed_pages,cnis.metadata.page_count)],
            vec!["Lançamentos originais de remuneração".into(),cnis.salary_entries.len().to_string()],
            vec!["Competências distintas da fonte".into(),months.len().to_string()],vec!["Competências com múltiplos lançamentos".into(),multi.to_string()],
            vec!["Vínculos originais / PRPPS".into(),format!("{} / {}",cnis.links.len(),cnis.links.iter().filter(|l|l.prpps).count())],
            vec!["Vínculos sem data final original".into(),cnis.links.iter().filter(|l|l.end_date.is_none()).count().to_string()],
            vec!["Eventos de benefício na fonte".into(),cnis.benefits.len().to_string()]]);
        s.paragraphs.push("Valores da fonte estão em centésimos nominais da unidade original (u.f.). Somar lançamentos não converte moedas históricas, não aplica teto nem determina salário RGPS, carência ou RMI.".into());
    }else{s.paragraphs.push("Sem CNIS anexado neste cálculo. A ausência da fonte não é conferência concluída.".into());}
    rows.extend([vec!["Períodos manuais / confirmados".into(),format!("{} / {}",case.manual_links.len(),case.manual_links.iter().filter(|l|l.confirmed).count())],
        vec!["Remunerações manuais / confirmadas".into(),format!("{} / {}",case.manual_salaries.len(),case.manual_salaries.iter().filter(|l|l.confirmed).count())]]);
    s.tables.push(table("Inventário do cenário",&["Item","Informação"],&[4,6],rows));
    if native.is_none(){s.paragraphs.push("RELATÓRIO DE CONFERÊNCIA / DADOS PENDENTES: nenhum resultado numérico de benefício foi fornecido. Não há RMI zero, concessão, regra vencedora ou equivalência integral a declarar.".into());}
    if options.include_personal {
        let mut rows=vec![vec!["Identificação do cálculo".into(),optional_field(Some(&case.name))?],vec!["Nome".into(),optional_field(case.identity.full_name.as_deref())?],
            vec!["CPF".into(),optional_field(case.identity.cpf.as_deref())?],vec!["NIT".into(),optional_field(case.identity.nit.as_deref())?],vec!["Nome da mãe".into(),optional_field(case.identity.mother_name.as_deref())?]];
        if let Some(cnis)=&case.cnis {rows.push(vec!["Identidade na fonte".into(),optional_field(cnis.identity.full_name.as_deref())?]);}
        s.tables.push(table("Identificação autorizada",&["Campo","Dado informado"],&[3,7],rows));
        if !case.notes.trim().is_empty(){s.paragraphs.push(format!("Nota livre do operador (não é política de cálculo): {}",field(&case.notes)?));}
    }
    Ok(s)
}
fn interview(case:&Calculation,personal:bool)->Result<Section,String>{
    let mut s=section("Entrevista do assistente e declarações",false);
    if let Some(a)=&case.assistant {
        let rows=vec![vec!["Modo".into(),enum_text(Some(&a.mode),&["assisted","manual"])],
            vec!["Recebe benefício atualmente?".into(),opt_bool(a.current_benefit_received).into()],vec!["Solicitou importação do CNIS?".into(),opt_bool(a.import_requested).into()],
            vec!["Tipo de processo".into(),enum_text(a.process_kind.as_deref(),&["Simulação do benefício","Revisão e valor da causa","Planejamento futuro","Processo administrativo","Execução judicial","Contribuições em atraso"])],
            vec!["Finalidade".into(),enum_text(a.goal.as_deref(),&["Calcular um novo benefício","Calcular somente o tempo de contribuição","Restabelecer um benefício"])],
            vec!["Revisão declarada pelo operador".into(),bool_text(a.review_confirmed)],vec!["Etapas registradas".into(),a.completed_steps.len().to_string()]];
        s.tables.push(table("Respostas registradas (não resposta ≠ não)",&["Pergunta","Resposta"],&[5,5],rows));
        let labels=[("intro","Introdução"),("benefit","Benefício atual"),("import","Importação"),("process","Processo"),("goal","Finalidade"),("parameters","Parâmetros"),("periods","Períodos"),("salaries","Remunerações"),("policy","Política"),("review","Revisão"),("result","Resultado")];
        let rows=labels.iter().map(|(key,label)|vec![label.to_string(),if a.completed_steps.iter().any(|s|s==key){"Visitada / registrada"}else{"Não registrada"}.into()]).collect();
        s.tables.push(table("Percurso do assistente",&["Etapa","Registro"],&[4,6],rows));
        s.paragraphs.push("Visitar uma etapa ou marcar revisão não comprova resolução dos indicadores, correção documental nem direito ao benefício. Respostas ausentes permanecem não respondidas.".into());
        if a.validate().is_err(){s.paragraphs.push("Entrevista contém valores fora do contrato; exige conferência e não certifica cálculo.".into());}
    }else{s.paragraphs.push("Entrevista do assistente não registrada (cálculo legado/manual); nenhuma resposta foi inferida.".into());}
    let mut rows=vec![vec!["Índice solicitado".into(),enum_text(Some(&case.options.correction_index),&["INPC","inpc",""])],vec!["Décimo terceiro solicitado".into(),bool_text(case.options.include_thirteenth)]];
    if case.options.rule_policy.len()>64*1024{return Err("Política declarada excede limite estrutural.".into());}
    let policy=serde_json::from_str::<Value>(&case.options.rule_policy).ok();
    if let Some(p)=&policy{json_budget(p)?; rows.extend([vec!["Fonte importada excluída no cenário?".into(),opt_bool(p.get("exclude_imported_source").and_then(Value::as_bool)).into()],
        vec!["Referência documental de revisão presente?".into(),bool_text(p.get("review_evidence").and_then(Value::as_str).is_some_and(|v|!v.trim().is_empty()))],
        vec!["Moeda manual declarada".into(),enum_text(p.get("manual_currency").and_then(Value::as_str),&["BRL","brl","source_nominal"])],
        vec!["Categoria manual declarada".into(),enum_text(p.get("manual_link_kind").and_then(Value::as_str),&["empregado","empregado_domestico","contribuinte_individual","avulso","facultativo","rural","especial","servidor"])] ]);
        if let Some(opts)=p.get("native_options"){
            for(key,label)in[("corrigirSalarios","Correção solicitada"),("descarte","Descarte solicitado"),("divisorMinimo","Divisor mínimo solicitado"),("aplicarTetoVigenteDer","Teto da referência solicitado")]{rows.push(vec![label.into(),opt_bool(opts.get(key).and_then(Value::as_bool)).into()]);}
            for(key,label)in[("teto","Teto manual solicitado"),("piso","Piso manual solicitado")]{rows.push(vec![label.into(),numeric(opts.get(key),true)?]);}
        }
        if personal {for(key,label)in[("review_evidence","Referência documental declarada"),("source_exclusion_evidence","Justificativa declarada para excluir fonte")]{if let Some(v)=p.get(key).and_then(Value::as_str){rows.push(vec![label.into(),field(v)?]);}}}
    }else{s.paragraphs.push("Política de cálculo estruturada não informada ou ilegível; nenhum texto livre foi convertido em regra automaticamente.".into());}
    s.tables.push(table("Política declarada — não substitui os parâmetros efetivamente usados",&["Campo","Declaração"],&[5,5],rows));
    Ok(s)
}
fn periods(case:&Calculation,personal:bool,native:Option<&Value>)->Result<Section,String>{
    let mut s=section("Períodos de contribuição",true);
    s.paragraphs.push("As datas abaixo são as reportadas na fonte. Fim ausente permanece ausente; candidato de último salário não é fim de vínculo. PRPPS/RPPS não é convertido para RGPS por declaração genérica.".into());
    if let Some(cnis)=&case.cnis {
        let mut rows=vec![];
        for l in &cnis.links {rows.push(vec![l.sequence.to_string(),if personal{optional_field(l.raw_employer.as_deref())?}else{format!("Vínculo {:02}",l.sequence)},
            date(l.start_date.as_deref()),date(l.end_date.as_deref()),if l.prpps{"PRPPS — pendente"}else{"Fonte não revisada"}.into(),
            indicators(&l.indicators,personal)?,source(&l.source)]);}
        s.tables.push(table("Todos os vínculos originais",&["Nº","Vínculo / empregador","Início","Fim original","Situação","Indicadores","Fonte"],&[1,4,2,2,3,3,2],rows));
        let mut rows=vec![];for l in &cnis.links {if let Some(candidate)=&l.inferred_last_salary_end{rows.push(vec![l.sequence.to_string(),month(&candidate.competence)?,date(Some(&candidate.date_candidate)),"Candidato não confirmado; não usado como fim original".into(),source(&candidate.source)]);}}
        if !rows.is_empty(){s.tables.push(table("Candidatos de fim — separados da informação original",&["Vínculo","Última competência","Data candidata","Alcance","Fonte"],&[1,2,2,5,2],rows));}
        let rows=cnis.benefits.iter().map(|b|vec![b.sequence.to_string(),b.species.map(|v|v.to_string()).unwrap_or("Não informada".into()),match b.status{BenefitStatus::Denied=>"Indeferido",BenefitStatus::Unknown=>"Situação desconhecida"}.into(),"Não prova recebimento atual".into(),source(&b.source)]).collect();
        if !cnis.benefits.is_empty(){s.tables.push(table("Eventos de benefício — não são vínculos de emprego",&["Nº","Espécie","Situação na fonte","Alcance","Fonte"],&[1,1,3,4,2],rows));}
    }
    let mut rows=vec![];for(i,l)in case.manual_links.iter().enumerate(){rows.push(vec![(i+1).to_string(),if personal{optional_field(Some(&l.employer))?}else{format!("Período manual {:02}",i+1)},date(Some(&l.start_date)),date(Some(&l.end_date)),enum_text(Some(&l.regime),&["RGPS","RPPS","PRPPS","rgps","rpps"]),bool_text(l.confirmed),indicators(&if l.indicators.is_empty(){vec![]}else{vec![l.indicators.clone()]},personal)?]);}
    s.tables.push(table("Declarações manuais — mantidas separadas da fonte",&["Nº","Vínculo","Início","Fim","Regime","Confirmado?","Indicadores"],&[1,4,2,2,2,2,3],rows));
    if let Some(t)=native.and_then(|v|v.get("tempo")){if !t.is_null(){
        let mut rows=vec![];for(key,label)in[("total_dias","Dias do motor"),("total_dias_efetivos","Dias efetivos do motor"),("carencia_meses","Carência do módulo de tempo")]{rows.push(vec![label.into(),numeric(t.get(key),false)?]);}
        for(key,label)in[("ate_ec20","Marco EC20 — dias"),("ate_ec103","Marco EC103 — dias"),("apos_ec103","Após EC103 — dias")]{rows.push(vec![label.into(),numeric(t.get("marcos").and_then(|v|v.get(key)),false)?]);}
        s.tables.push(table("Tempo computacional experimental",&["Medida","Resultado fornecido"],&[6,4],rows));
        if let Some(vs)=t.get("vinculos").and_then(Value::as_array){let mut rows=vec![];for v in vs{rows.push(vec![numeric(v.get("indice"),false)?,date(v.get("inicio").and_then(Value::as_str)),date(v.get("fim").and_then(Value::as_str)),numeric(v.get("dias"),false)?,numeric(v.get("dias_efetivos"),false)?,opt_bool(v.get("concomitante").and_then(Value::as_bool)).into(),opt_bool(v.get("sobreposto").and_then(Value::as_bool)).into()]);}
            s.tables.push(table("Períodos processados pelo motor (não substituem a fonte)",&["Nº","Início","Fim usado","Dias","Dias efetivos","Concomitante","Sobreposto"],&[1,2,2,2,2,2,2],rows));}
    }} Ok(s)
}
fn salaries(case:&Calculation,personal:bool)->Result<Vec<Section>,String>{
    let mut s=section("Salários de contribuição",true);
    s.paragraphs.push("u.f. = unidade nominal da fonte. A soma mensal é uma conferência aritmética, não soma de salários atuais em BRL. Concomitância e moedas anteriores ao real exigem política e revisão. Não há corte dos registros para caber em uma página.".into());
    let mut detail=section("Relação das remunerações",true);
    detail.paragraphs.push("Cada linha importada mantém vínculo, página, indicadores e natureza da coluna. Valores anteriores a julho/1994 são preservados para conferência, não incluídos automaticamente no PBC.".into());
    if let Some(cnis)=&case.cnis{
        let mut groups:BTreeMap<&str,(i128,usize,bool,BTreeSet<u32>,BTreeSet<String>)>=BTreeMap::new();
        let mut rows=vec![];
        for(i,e)in cnis.salary_entries.iter().enumerate(){month(&e.competence)?;let g=groups.entry(&e.competence).or_default();g.0=g.0.checked_add(i128::from(e.amount_centavos)).ok_or("Overflow de soma nominal.")?;g.1+=1;g.2|=e.excluded_from_validated_domain;g.3.insert(e.source.page);g.4.extend(e.indicators.iter().cloned());
            rows.push(vec![(i+1).to_string(),month(&e.competence)?,e.link_sequence.map(|s|s.to_string()).unwrap_or("Não atribuído".into()),cents(i128::from(e.amount_centavos)),
                match e.amount_kind{AmountKind::Remuneration=>"Remuneração",AmountKind::ContributionColumnUnresolved=>"Coluna não resolvida"}.into(),indicators(&e.indicators,personal)?,source(&e.source)]);
        }
        let mut grouped=vec![];for(m,(sum,count,excluded,pages,flags))in groups{grouped.push(vec![month(m)?,count.to_string(),cents(sum),if count>1{"Múltiplos lançamentos"}else{"Um lançamento"}.into(),if excluded{"Fora do domínio validado"}else{"Fonte não revisada"}.into(),indicators(&flags.into_iter().collect::<Vec<_>>(),personal)?,pages.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")]);}
        s.tables.push(table("Todas as competências distintas",&["Competência","Linhas","Soma nominal (u.f.)","Conferência","Domínio","Indicadores","Páginas"],&[2,1,3,3,3,3,1],grouped));
        detail.tables.push(table("Todos os lançamentos importados",&["Nº","Competência","Vínculo","Valor nominal (u.f.)","Natureza","Indicadores","Fonte"],&[1,2,1,3,3,3,2],rows));
    }else{s.paragraphs.push("Não há remunerações importadas de CNIS neste cenário.".into());}
    let mut rows=vec![];for(i,m)in case.manual_salaries.iter().enumerate(){rows.push(vec![(i+1).to_string(),month(&m.competence)?,manual_amount(&m.amount)?,bool_text(m.confirmed),indicators(&if m.indicators.trim().is_empty(){vec![]}else{vec![m.indicators.clone()]},personal)?]);}
    detail.tables.push(table("Remunerações manuais — moeda depende da política declarada",&["Nº","Competência","Valor informado","Confirmada?","Indicadores"],&[1,2,3,2,4],rows));
    Ok(vec![s,detail])
}
fn pending_label(code:&str)->Option<&'static str>{
    match code {
        "currency_units_unverified"=>Some("Unidades monetárias nominais da fonte não conferidas/conversões históricas não aplicadas."),
        "source_review_required"|"review_not_confirmed"=>Some("Revisão documental por registro ainda exigida."),
        "prpps_unvalidated"=>Some("Vínculos PRPPS/RPPS não validados como RGPS; CTC não presumida."),
        "source_end_missing"|"missing_or_invalid_date"=>Some("Data ausente/inválida exige confirmação documental."),
        "source_indicators_unresolved"=>Some("Indicadores/avisos de origem não resolvidos."),
        "inpc_out_of_coverage"=>Some("Índice solicitado fora da cobertura; sem extrapolação."),
        "source_exclusion_not_documented"=>Some("Exclusão de fonte não documentada."),
        "sex_unconfirmed"|"disability_unconfirmed"=>Some("Parâmetros pessoais do segurado não confirmados."),
        "case_policy_missing"|"policy_not_selected"=>Some("Política versionada não informada."),
        "case_review_missing"=>Some("Referência de revisão documental ausente."),
        "fator_previdenciario_unvalidated"=>Some("Caminho de fator previdenciário bloqueado por divergências conhecidas."),
        _=>None,
    }
}
fn pending(case:&Calculation,personal:bool,outcome:Option<&Value>)->Result<Section,String>{
    let mut s=section("Indicadores, pendências e ressalvas",true);
    s.paragraphs.push("A importação e a visita às telas não equivalem à revisão previdenciária. Finais ausentes, PRPPS, concomitâncias, valores mínimos/tetos e precisão monetária devem permanecer pendentes até conferência documentada.".into());
    let mut rows=vec![];
    if let Some(cnis)=&case.cnis {
        for l in &cnis.links{if l.prpps||l.end_date.is_none()||!l.indicators.is_empty(){rows.push(vec![format!("Vínculo {}",l.sequence),if l.prpps{"PRPPS; não RGPS validado"}else{"Fonte não revisada"}.into(),if l.end_date.is_none(){"Fim original ausente"}else{"Fim original presente"}.into(),indicators(&l.indicators,personal)?,source(&l.source)]);}}
        s.tables.push(table("Pendências por vínculo",&["Registro","Domínio","Data final","Indicadores","Fonte"],&[2,4,3,4,2],rows));
        let mut rows=vec![];for(i,e)in cnis.salary_entries.iter().enumerate(){if !e.indicators.is_empty()||e.excluded_from_validated_domain||e.amount_kind==AmountKind::ContributionColumnUnresolved{rows.push(vec![(i+1).to_string(),month(&e.competence)?,indicators(&e.indicators,personal)?,if e.excluded_from_validated_domain{"Não validado; excluído"}else{"Fonte não revisada"}.into(),source(&e.source)]);}}
        s.tables.push(table("Pendências por remuneração",&["Linha","Competência","Indicadores","Situação","Fonte"],&[1,2,4,4,2],rows));
        let mut rows=vec![];for(findex,f)in cnis.findings.iter().chain(cnis.warnings.iter()).enumerate(){let text=if personal{field(&f.message)?}else{pending_label(&f.code).unwrap_or("Achado técnico de importação; mensagem livre oculta neste modo.").to_string()}; rows.push(vec![(findex+1).to_string(),if pending_label(&f.code).is_some(){f.code.clone()}else{format!("Achado {:02}",findex+1)},text,f.count.to_string(),f.source.as_ref().map(source).unwrap_or("Geral".into())]);}
        s.tables.push(table("Achados e avisos da extração",&["Nº","Tipo técnico","Descrição / alcance","Ocorrências","Fonte"],&[1,3,7,2,2],rows));
    }
    if let Some(o)=outcome{
        let mut rows=vec![];
        for(key,label)in[("blockers","Bloqueio"),("missing","Dado pendente"),("warnings","Aviso"),("known_model_risks","Risco conhecido")]{if let Some(vs)=o.get(key).and_then(Value::as_array){for(i,v)in vs.iter().enumerate(){let code=v.get("code").and_then(Value::as_str).or_else(||v.as_str());let message=if personal{v.get("message").and_then(Value::as_str).or_else(||v.as_str()).map(field).transpose()?.unwrap_or_else(||"Estrutura de aviso não reconhecida; conteúdo não publicado.".into())}else{code.and_then(pending_label).unwrap_or("Mensagem técnica livre oculta; consultar pendências na aplicação autorizada.").into()};rows.push(vec![format!("{label} {}",i+1),message]);}}}
        s.tables.push(table("Guardas, avisos e riscos do resultado explicitamente fornecido",&["Categoria","Ressalva"],&[3,7],rows));
        if let Some(ns)=o.pointer("/data/native_result/avisos").and_then(Value::as_array){let rows=ns.iter().enumerate().map(|(i,v)|Ok(vec![format!("Aviso nativo {}",i+1),if personal{optional_field(v.as_str())?}else{"Texto livre do motor oculto. Resultado continua experimental e sem certificação jurídica.".into()}])).collect::<Result<Vec<_>,String>>()?;s.tables.push(table("Avisos da execução nativa",&["Registro","Ressalva"],&[3,7],rows));}
    }
    s.paragraphs.push("Limitações do motor: transições utilizam simplificações de competências e marcos; correção depende da cobertura do snapshot; fator previdenciário e combinações não suportadas devem continuar bloqueados. Estes avisos não constituem fundamentação jurídica.".into());
    Ok(s)
}
fn engine(native:Option<&Value>,personal:bool)->Result<Vec<Section>,String>{
    let mut sum=section("Cálculo da renda mensal inicial",true);
    let mut memory=section("Memória de cálculo",true);
    let mut correction=section("Salários de contribuição - memória de atualização",true);
    let Some(n)=native else{sum.paragraphs.push("Resultado indisponível: não foi fornecida execução experimental revisada. Sem salário de benefício, coeficiente aplicado ou RMI inventados.".into());memory.paragraphs.push("Nenhuma memória de cálculo fornecida; apenas conferência dos dados.".into());correction.paragraphs.push("Nenhuma série de correção fornecida; não foram inventados índices ou parcelas.".into());return Ok(vec![sum,memory,correction]);};
    sum.paragraphs.push("RESULTADO EXPERIMENTAL - valores da hipótese revisada. Conferir regras, documentos e parâmetros antes do uso profissional.".into());
    let r=n.get("resumo"); let mut rows=vec![];
    for(key,label,money)in[("competencias","Salários considerados",false),("carencia_meses","Carência apurada (meses)",false),("soma_salarios","Soma dos salários considerados",true),("salario_beneficio","Salário de benefício",true),("coeficiente","Coeficiente aplicado (%)",false),("rmi_bruta","Renda mensal antes dos limites",true),("rmi","Renda mensal inicial (experimental)",true),("teto","Teto aplicado",true),("piso","Piso aplicado",true)]{rows.push(vec![label.into(),numeric(r.and_then(|r|r.get(key)),money)?]);}
    sum.tables.push(table("Demonstrativo da renda mensal inicial",&["Medida","Valor fornecido"],&[6,4],rows));
    let p=n.get("parametros");let mut rows=vec![];for(key,label)in[("divisor","Divisor usado"),("anos","Anos computados"),("meses_restantes","Meses restantes"),("anos_base","Anos-base"),("coeficiente","Coeficiente (%)")]{rows.push(vec![label.into(),numeric(p.and_then(|p|p.get(key)),false)?]);}
    for(key,label)in[("divisor_minimo_aplicado","Divisor mínimo aplicado?"),("correcao_ativa","Correção ativa?"),("descarte_aplicado","Descarte aplicado?")]{rows.push(vec![label.into(),opt_bool(p.and_then(|p|p.get(key)).and_then(Value::as_bool)).into()]);}
    if let Some(reference)=p.and_then(|p|p.get("referencia_correcao")).and_then(Value::as_str){rows.push(vec!["Competência de correção".into(),month(reference)?]);}
    sum.tables.push(table("Parâmetros utilizados no cálculo",&["Parâmetro","Valor do motor"],&[6,4],rows));
    let mut rows=vec![];
    if let Some(items)=n.get("memoria").and_then(Value::as_array){for(i,v)in items.iter().enumerate(){let kind=v.get("tipo").and_then(Value::as_str);let known=match kind{Some("media")=>Some("Média / salário de benefício"),Some("coeficiente")=>Some("Coeficiente computacional"),Some("rmi")=>Some("Salário de benefício × coeficiente"),Some("limite")=>Some("Aplicação de piso ou teto"),_=>None};let value=v.get("valor").and_then(Value::as_str);let number=match value{Some(v)=>if let Some(v)=v.strip_suffix('%'){format!("{}%",decimal(v,false)?)}else{decimal(v,true)?},None=>"Não produzido".into()};
        rows.push(vec![(i+1).to_string(),known.unwrap_or("Etapa não catalogada").into(),if personal{optional_field(v.get("descricao").and_then(Value::as_str))?}else{known.unwrap_or("Descrição livre oculta").into()},number]);}}
    memory.tables.push(table("Todas as etapas numéricas retornadas",&["Nº","Etapa","Descrição","Valor fornecido"],&[1,3,6,3],rows));
    if let Some(c)=n.get("carencia"){let mut rows=vec![];for(key,label)in[("competencias_salario","Competências de remuneração"),("competencias_vinculo","Competências de vínculos"),("competencias_graca","Competências de graça"),("recolhimentos_atraso","Recolhimentos em atraso"),("carencia_meses","União de competências — meses")]{rows.push(vec![label.into(),numeric(c.get(key),false)?]);}memory.tables.push(table("Decomposição da carência computacional",&["Origem","Quantidade fornecida"],&[6,4],rows));}
    if let Some(c)=n.get("correcao").filter(|v|!v.is_null()){
        let mut rows=vec![];for(key,label,money)in[("pbc","Parcelas no PBC",false),("consideradas","Parcelas consideradas",false),("descartadas","Parcelas descartadas",false),("soma_considerada","Soma considerada",true),("media","Média computacional",true),("salario_beneficio","Salário de benefício",true)]{rows.push(vec![label.into(),numeric(c.get(key),money)?]);}
        if let Some(v)=c.get("referencia").and_then(Value::as_str){rows.push(vec!["Referência de correção".into(),month(v)?]);}
        correction.tables.push(table("Síntese da correção",&["Medida","Valor"],&[6,4],rows));
        let mut rows=vec![];let mut sources:BTreeMap<String,usize>=BTreeMap::new();let discarded:BTreeSet<_>=c.get("parcelas_descartadas").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
        if let Some(items)=c.get("parcelas").and_then(Value::as_array){for v in items{let comp=v.get("competencia").and_then(Value::as_str).ok_or("Parcela sem competência.")?;
            let source_id=if personal{let f=optional_field(v.get("fonte").and_then(Value::as_str))?;let next=sources.len()+1;*sources.entry(f).or_insert(next)}else{1};
            rows.push(vec![month(comp)?,numeric(v.get("salario"),true)?,numeric(v.get("teto"),true)?,numeric(v.get("considerado"),true)?,numeric(v.get("indice"),false)?,numeric(v.get("corrigido"),true)?,if discarded.contains(comp){"Descartada"}else{"Considerada"}.into(),format!("F{source_id}")]);}}
        correction.tables.push(table("Todas as parcelas de correção do resultado",&["Competência","Nominal","Teto época","Considerado","Fator","Corrigido","Uso no PBC","Fonte"],&[2,2,2,2,2,2,2,1],rows));
        let rows=if personal{sources.iter().map(|(s,id)|vec![format!("F{id}"),s.clone()]).collect()}else{vec![vec!["F1".into(),"Fonte textual do motor oculta no modo sem identificação; proveniência e cobertura exigem conferência na aplicação.".into()]]};
        correction.tables.push(table("Referências declaradas pelo resultado — não certificadas pelo renderer",&["ID","Descrição"],&[1,9],rows));
    }else{correction.paragraphs.push("Resultado não contém correção parcela a parcela. Sem série de índices fabricada; média nominal somente se explicitamente retornada pelo motor.".into());}
    for(key,label)in[("parametros","Parâmetros"),("resumo","Resumo"),("tempo","Tempo"),("carencia","Carência"),("correcao","Correção"),("memoria","Memória"),("transicoes","Transições")]{if let Some(v)=n.get(key){let known:&[&str]=match key{
        "resumo"=>&["competencias","carencia_meses","soma_salarios","salario_beneficio","coeficiente","rmi_bruta","rmi","limite_aplicado","teto","piso"],
        "parametros"=>&["divisor","divisor_minimo_aplicado","anos","meses_restantes","anos_base","coeficiente","correcao_ativa","referencia_correcao","descarte_aplicado"],
        "tempo"=>&["vinculos","total_dias","total_dias_efetivos","total","carencia_meses","marcos","avisos"],
        "carencia"=>&["competencias_salario","competencias_vinculo","competencias_graca","recolhimentos_atraso","carencia_meses","competencias_extras","avisos"],
        "correcao"=>&["referencia","pbc","consideradas","descartadas","parcelas","parcelas_descartadas","soma_considerada","media","salario_beneficio","descarte_aplicado","fontes_tetos","fontes_inpc"],
        "transicoes"=>&["referencia","sexo","idade_meses","idade_anos","tempo_total_meses","tempo_total_anos","tempo_ate_reforma_meses","regras","regra_escolhida","numero_regra_transicao","rmi_escolhida","observacoes"],_=>&[]};
        if let Some(object)=v.as_object(){let count=object.keys().filter(|k|!known.contains(&k.as_str())).count();if count>0{memory.paragraphs.push(format!("{label}: {count} campo(s) fora do contrato documental não publicado(s), para evitar expor dados arbitrários do resultado."));}}
    }}
    Ok(vec![sum,memory,correction])
}
fn rule_label(id:Option<&str>)->&'static str{match id{Some("direito-adquirido")=>"Direito adquirido (pré-reforma)",Some("pontos")=>"Transição por pontos",Some("idade-progressiva")=>"Idade progressiva",Some("pedagio-50")=>"Pedágio de 50%",Some("pedagio-100")=>"Pedágio de 100%",Some("idade-permanente")=>"Regra por idade permanente",_=>"Regra não catalogada"}}
fn transitions(native:Option<&Value>,personal:bool)->Result<Section,String>{
    let mut s=section("Análise das regras de transição",true);
    s.paragraphs.push("Indicações de elegibilidade abaixo são apenas saídas computacionais do motor experimental. Simplificações de competências e marcos impedem apresentá-las como direito comprovado. RMI ausente não é zero.".into());
    let Some(t)=native.and_then(|n|n.get("transicoes")).filter(|v|!v.is_null())else{s.paragraphs.push("Comparador não foi produzido neste cenário. Sem regras aprovadas ou vencedor inventado.".into());return Ok(s);};
    let mut rows=vec![];for(key,label)in[("idade_meses","Idade em meses"),("idade_anos","Idade em anos"),("tempo_total_meses","Tempo em meses usado pelo comparador"),("tempo_ate_reforma_meses","Meses até reforma usados"),("numero_regra_transicao","Número da regra selecionada")]{rows.push(vec![label.into(),numeric(t.get(key),false)?]);}
    rows.push(vec!["Regra computacional escolhida".into(),if t.get("regra_escolhida").is_none_or(Value::is_null){"Não produzida".into()}else{rule_label(t.get("regra_escolhida").and_then(Value::as_str)).into()}]);
    rows.push(vec!["RMI escolhida pelo comparador (não jurídica)".into(),numeric(t.get("rmi_escolhida"),true)?]);s.tables.push(table("Entradas e seleção do comparador",&["Medida","Resultado fornecido"],&[6,4],rows));
    if let Some(rules)=t.get("regras").and_then(Value::as_array){let mut rows=vec![];for(i,r)in rules.iter().enumerate(){rows.push(vec![(i+1).to_string(),rule_label(r.get("id").and_then(Value::as_str)).into(),opt_bool(r.get("elegivel").and_then(Value::as_bool)).into(),enum_text(r.get("criterio_rmi").and_then(Value::as_str),&["regras-antigas","coeficiente-progressivo","100%-media","fator-previdenciario"]),numeric(r.get("rmi"),true)?,numeric(r.get("rmi_final"),true)?]);}
        s.tables.push(table("Todas as regras retornadas",&["Nº","Regra","Critério atingido?*","Critério da RMI","RMI estimada","RMI após limites"],&[1,4,3,4,3,3],rows));
        for(i,r)in rules.iter().enumerate(){let mut rows=vec![];if let Some(reqs)=r.get("requisitos").and_then(Value::as_array){for(j,req)in reqs.iter().enumerate(){rows.push(vec![format!("Requisito {}",j+1),if personal{optional_field(req.as_str())?}else{"Texto livre do requisito oculto; ver dados numéricos da comparação e conferir na aplicação.".into()}]);}}
            if let Some(f)=r.get("faltas").and_then(Value::as_object){for(key,label)in[("tempoMeses","Tempo faltante (meses)"),("pontos","Pontos faltantes"),("idadeMeses","Idade faltante (meses)"),("pedagioMeses","Pedágio faltante (meses)")]{if f.contains_key(key){rows.push(vec![label.into(),numeric(f.get(key),false)?]);}}let unknown=f.keys().filter(|k|!["tempoMeses","pontos","idadeMeses","pedagioMeses"].contains(&k.as_str())).count();if unknown>0{rows.push(vec!["Campos não publicados".into(),format!("{unknown} campo(s) de estrutura não reconhecida; nenhum conteúdo arbitrário divulgado")]);}}
            if personal{if let Some(obs)=r.get("observacao").and_then(Value::as_str){rows.push(vec!["Observação do motor".into(),field(obs)?]);}}
            if !rows.is_empty(){s.tables.push(table(&format!("Regra {} — requisitos e faltas",i+1),&["Item","Detalhe fornecido"],&[4,6],rows));}
        }
    }
    s.paragraphs.push("*Critério atingido = flag elegivel do motor, não concessão nem conclusão jurídica. Vencedor do comparador pode diferir do coeficiente/resumo do motor; ambos são preservados sem forçar igualdade.".into());Ok(s)
}
pub(crate) fn build(case:&Calculation,workspace:&Workspace,options:ReportOptions,outcome:Option<&Value>)->Result<Report,String>{
    if let Some(v)=outcome{json_budget(v)?;}
    if options.include_personal { field(&case.name)?; }
    let input_count=case.manual_links.len().checked_add(case.manual_salaries.len()).and_then(|v|v.checked_add(case.cnis.as_ref().map_or(0,|c|c.links.len()+c.salary_entries.len()+c.findings.len()+c.warnings.len()))).ok_or("Quantidade de registros excede limite.")?;
    if input_count>MAX_ROWS{return Err("Entrada excede 10.000 registros; nenhum registro foi omitido para caber no relatório.".into());}
    let native=outcome.filter(|v|v.get("status").and_then(Value::as_str)==Some("experimental")).and_then(|v|v.pointer("/data/native_result")).filter(|v|v.is_object());
    let mut report=Report{title:options.kind.title().into(),status:if native.is_some(){"RESULTADO EXPERIMENTAL / REGRAS NÃO CERTIFICADAS"}else{"RELATÓRIO DE CONFERÊNCIA / DADOS PENDENTES"}.into(),
        office:if options.include_personal{let office=workspace.settings.office_name.trim();if office.is_empty(){"PREVIUS RUST • RELATÓRIOS".into()}else{field(office)?}}else{"Escritório previdenciário".into()},
        footer:if options.include_personal{let f=workspace.settings.report_footer.trim();if f.is_empty(){"Uso confidencial. Revisão documental e normativa necessária.".into()}else{field(f)?}}else{"Identificação oculta. Conferir dados, documentos e parâmetros.".into()},personal:options.include_personal,identification: [
            format!("Nome do segurado: {}", if options.include_personal { format!("{}    Cálculo: {}", optional_field(case.identity.full_name.as_deref())?, field(&case.name)?) } else { "Identificação oculta".into() }),
            format!("CPF: {}    Nascimento: {}", if options.include_personal { optional_field(case.identity.cpf.as_deref())? } else { "Oculto".into() }, date(case.identity.birth_date.as_deref())),
            format!("DIB: {}    Sexo: {}    Hipótese sujeita a revisão", date(Some(&case.dib)), enum_text(case.sex.as_deref(), &["masculino", "feminino", "Masculino", "Feminino"]))
        ],sections:vec![]};
    if report.office.chars().count()>120 || report.footer.chars().count()>180{return Err("Cabeçalho/rodapé excede o espaço reservado; nenhum texto foi cortado.".into());}
    report.sections.push(summary(case,options,native)?);
    if options.kind == ReportKind::Detailed { report.sections.push(interview(case,options.include_personal)?); }
    if selected(options,&[ReportKind::Periods]){report.sections.push(periods(case,options.include_personal,native)?);}
    if selected(options,&[ReportKind::Salaries]) || (options.kind == ReportKind::Memory && native.and_then(|n| n.get("correcao")).is_none_or(Value::is_null)) { report.sections.extend(salaries(case,options.include_personal)?.into_iter().filter(|s| options.kind != ReportKind::Memory || s.tables.iter().any(|t| !t.rows.is_empty()))); }
    if selected(options,&[ReportKind::Memory,ReportKind::Summary]){let mut sections=engine(native,options.include_personal)?;if options.kind==ReportKind::Summary{sections.truncate(1);}report.sections.extend(sections);}
    if selected(options,&[ReportKind::Transitions]){report.sections.push(transitions(native,options.include_personal)?);}
    // Summary has a compact warning cover; all other kinds include relevant pending records.
    if options.kind==ReportKind::Summary {let mut s=section("Ressalvas e conclusão",false);s.paragraphs.push("A fonte não foi automaticamente validada. Conferir PRPPS, datas finais, concomitância, indicadores e moedas históricas antes de usar um resultado para benefício.".into());if let Some(o)=outcome{let count=["blockers","missing","warnings","known_model_risks"].iter().map(|k|o.get(k).and_then(Value::as_array).map_or(0,Vec::len)).sum::<usize>();s.paragraphs.push(format!("Resultado fornecido registra {count} pendência(s)/aviso(s)/risco(s). O relatório detalhado mantém as categorias e origens."));}report.sections.push(s);}
    else{report.sections.push(pending(case,options.include_personal,outcome)?);}
    // Domain reports lead with their own content. Import inventory belongs to the appendix.
    let mut inventory = report.sections.remove(0);
    inventory.title = "Anexo técnico - dados, origem e alcance".into();
    inventory.new_page = true;
    report.sections.sort_by_key(|s| {
        if options.kind == ReportKind::Transitions {
            return if s.title == "Análise das regras de transição" { 0 } else { 1 };
        }
        if options.kind == ReportKind::Memory {
            return match s.title.as_str() {
                "Salários de contribuição - memória de atualização" if s.tables.iter().any(|t| !t.rows.is_empty()) => 0,
                "Cálculo da renda mensal inicial" => 1,
                "Memória de cálculo" => 2,
                _ if s.title.to_lowercase().contains("remunerações") || s.title.to_lowercase().contains("salários de contribuição") => 0,
                _ => 3,
            };
        }
        match s.title.as_str() {
            "Cálculo da renda mensal inicial" => 0,
            "Períodos de contribuição" => 1,
            "Salários de contribuição - memória de atualização" => 2,
            "Análise das regras de transição" => 4,
            "Memória de cálculo" => 5,
            "Entrevista do assistente e declarações" => 6,
            _ => 3,
        }
    });
    if options.kind == ReportKind::Detailed { report.sections.push(inventory); }
    for s in &mut report.sections { s.tables.retain(|t| !t.rows.is_empty()); }
    report.sections.retain(|s| !s.paragraphs.is_empty() || !s.tables.is_empty());
    if let Some(first) = report.sections.first_mut() { first.new_page = false; }
    for line in &report.identification { field(line)?; }
    let mut total_rows=0usize;
    for s in &report.sections{for t in &s.tables{if t.headers.len()!=t.weights.len()||t.weights.contains(&0){return Err("Definição de tabela PDF inválida.".into());}for r in &t.rows{if r.len()!=t.headers.len(){return Err("Linha de tabela PDF inválida.".into());}total_rows+=1;for v in r{field(v)?;}}}for p in &s.paragraphs{field(p)?;}}
    if total_rows>MAX_ROWS{return Err("Documento excede 10.000 linhas de tabela; nenhum registro foi omitido.".into());}
    Ok(report)
}
#[cfg(test)]mod tests{use super::*;
    #[test]fn exact_currency_not_float(){assert_eq!(cents(326940),"3.269,40");assert_eq!(cents(-1),"-0,01");assert_eq!(decimal("2048.18",true).unwrap(),"2.048,18");assert!(decimal("1.123",true).is_err());}
    #[test]fn null_bool_is_distinct(){assert_ne!(opt_bool(None),opt_bool(Some(false)));assert_eq!(opt_bool(None),"Não respondido");}
    #[test]fn date_is_checked(){assert!(iso_valid("2024-02-29"));assert!(!iso_valid("2023-02-29"));assert!(!iso_valid("1900-02-29"));assert!(!iso_valid("敏-01-01"));}
    #[test]fn text_limits_fail_not_clip(){assert!(field(&"a".repeat(1001)).is_err());assert!(field("a\0b").is_err());}
    #[test]fn numeric_not_arbitrary_text(){assert!(numeric(Some(&Value::String("CPF123".into())),true).is_err());}
    #[test]fn private_indicator_whitelist(){assert_eq!(indicators(&["PRPPS".into(),"SECRET NAME".into()],false).unwrap(),"PRPPS; 1 código(s) não catalogado(s) oculto(s)");}
}
