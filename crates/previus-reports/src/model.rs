use crate::{ReportKind, ReportOptions};
use previus_cnis::{AmountKind, BenefitStatus, SourceRef};
use previus_workspace::{Calculation, Workspace};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
const MAX_ROWS: usize = 10_000;
const MAX_FIELD: usize = 1_000;
#[derive(Debug)] pub(crate) struct Report { pub title: String, pub status: String, pub office: String, pub footer: String, pub personal: bool, pub identification: [String; 3], pub sections: Vec<Section>, pub highlight: Option<String> }
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
fn declared_disability(value: Option<&str>) -> String {
    match value.map(|s|s.trim().to_lowercase()).as_deref() {
        Some("nao"|"não"|"none"|"sem_deficiencia"|"não possui deficiência"|"nao possui deficiencia"|"nenhuma")=>"Sem deficiência declarada".into(),
        Some("leve")=>"Leve".into(),Some("moderada")=>"Moderada".into(),Some("grave")=>"Grave".into(),
        None=>"Não informado".into(),_=>"Valor fora do contrato / pendente".into(),
    }
}
fn policy_value(case:&Calculation)->Option<Value>{
    let raw=case.options.rule_policy.trim();
    if raw.is_empty() || !raw.starts_with('{') {return None;}
    serde_json::from_str(raw).ok()
}
fn policy_known(policy:Option<&Value>,key:&str,allowed:&[&str])->String{
    policy.and_then(|p|p.get(key)).and_then(Value::as_str)
        .filter(|value|allowed.contains(value)).unwrap_or("Não declarada / fora do contrato").into()
}
fn policy_optional(policy:Option<&Value>,key:&str)->Option<String>{
    policy.and_then(|p|p.get(key)).and_then(Value::as_str).map(str::trim).filter(|value|!value.is_empty()).map(str::to_owned)
}
fn considered_rows(case:&Calculation,personal:bool,native:bool)->Result<Vec<Vec<String>>,String>{
    let policy=policy_value(case);let p=policy.as_ref();
    let excluded=p.and_then(|v|v.get("exclude_imported_source")).and_then(Value::as_bool).unwrap_or(false);
    let exclusion_evidence=policy_optional(p,"source_exclusion_evidence");
    let review_evidence=policy_optional(p,"review_evidence");
    let native_options=p.and_then(|v|v.get("native_options"));
    let selected=case.manual_links.iter().filter(|link|!link.source_review.as_ref().is_some_and(|review|review.excluded)).count();
    let confirmed=case.manual_links.iter().filter(|link|!link.source_review.as_ref().is_some_and(|review|review.excluded)&&link.confirmed).count();
    let excluded_links=case.manual_links.iter().filter(|link|link.source_review.as_ref().is_some_and(|review|review.excluded)).count();
    let months:BTreeSet<_>=case.manual_salaries.iter().map(|salary|salary.competence.as_str()).collect();
    let mut rows=vec![];
    if let Some(cnis)=&case.cnis{
        rows.push(vec!["Fonte CNIS".into(),if excluded{"Excluída do motor; documento preservado".into()}else{"Anexada; o motor não a usa até exclusão/revisão documentada".into()}]);
        rows.push(vec!["Justificativa da exclusão da fonte".into(),match(&excluded,&exclusion_evidence){
            (true,Some(value)) if personal=>field(value)?,(true,Some(_))=>"Registrada".into(),(true,None)=>"Ausente".into(),_=>"Não aplicável".into()}]);
        rows.push(vec!["Vínculos na fonte".into(),format!("{}; {} PRPPS",cnis.links.len(),cnis.links.iter().filter(|link|link.prpps).count())]);
        rows.push(vec!["Lançamentos na fonte".into(),cnis.salary_entries.len().to_string()]);
    }else{rows.push(vec!["Fonte CNIS".into(),"Não anexada".into()]);}
    rows.push(vec!["Vínculos manuais usados".into(),format!("{selected} selecionado(s); {confirmed} confirmado(s); {excluded_links} excluído(s)")]);
    rows.push(vec!["Remunerações manuais usadas".into(),format!("{}; {} confirmada(s)",case.manual_salaries.len(),case.manual_salaries.iter().filter(|salary|salary.confirmed).count())]);
    rows.push(vec!["Competências distintas".into(),months.len().to_string()]);
    rows.push(vec!["Livro de contribuições".into(),match &case.contribution_review{
        Some(review)=>format!("{} lançamento(s); {} excluído(s); {}",review.records.len(),review.records.iter().filter(|record|record.excluded).count(),if review.applied{"aplicado"}else{"não aplicado"}),
        None=>"Não usado".into()}]);
    rows.push(vec!["Política".into(),policy_known(p,"version",&["rgps-core-observational-v1"])]);
    rows.push(vec!["Referência de revisão".into(),match review_evidence{
        Some(value) if personal=>field(&value)?,Some(_)=>"Registrada".into(),None=>"Ausente".into()}]);
    rows.push(vec!["Categoria dos períodos".into(),policy_known(p,"manual_link_kind",&["empregado","empregado_domestico","contribuinte_individual","avulso","facultativo"])]);
    rows.push(vec!["Moeda dos salários".into(),policy_known(p,"manual_currency",&["BRL"])]);
    for(key,label)in[("corrigirSalarios","Correção dos salários"),("descarte","Descarte"),("divisorMinimo","Divisor mínimo"),("aplicarTetoVigenteDer","Teto vigente derivado")]{
        rows.push(vec![label.into(),opt_bool(native_options.and_then(|v|v.get(key)).and_then(Value::as_bool)).into()]);
    }
    if let Some(reference)=native_options.and_then(|v|v.get("referenciaCorrecao")).and_then(Value::as_str).map(str::trim).filter(|value|!value.is_empty()){
        rows.push(vec!["Referência de correção".into(),if personal{field(reference)?}else{"Declarada".into()}]);
    }
    if let Some(value)=native_options.and_then(|v|v.get("expectativaSobrevida")).and_then(Value::as_str).filter(|v|!v.trim().is_empty()){
        rows.push(vec!["Expectativa de sobrevida".into(),if personal{field(value)?}else{"Declarada; caminho de fator bloqueado".into()}]);
    }
    for(key,label)in[("teto","Teto manual"),("piso","Piso manual")]{
        if let Some(value)=native_options.and_then(|v|v.get(key)).filter(|v|!v.is_null()){
            rows.push(vec![label.into(),numeric(Some(value),true)?]);
        }
    }
    rows.push(vec!["Índice de correção".into(),if case.options.correction_index.trim().is_empty(){"Não informado".into()}else if personal{field(&case.options.correction_index)?}else{"Declarado".into()}]);
    rows.push(vec!["Resultado".into(),if native{"Experimental fornecido pelo motor; não é concessão".into()}else{"Nenhum resultado financeiro produzido".into()}]);
    rows.push(vec!["Validação jurídica".into(),"Não executada".into()]);
    Ok(rows)
}
fn conclusion(case:&Calculation,options:ReportOptions,native:Option<&Value>,outcome:Option<&Value>)->Result<Section,String>{
    let personal=options.include_personal;
    let mut s=section("Conclusão e o que pode mudá-la",false);
    if let Some(n)=native{
        let rmi=numeric(n.pointer("/resumo/rmi"),true)?;
        if rmi=="Não produzido"{s.paragraphs.push("O motor foi executado, mas não produziu uma RMI para esta hipótese. Nenhum valor substituto foi inventado.".into());}
        else{s.paragraphs.push(format!("Hipótese calculada pelo motor experimental. Renda mensal inicial estimada: R$ {rmi}. Este valor não é concessão, não certifica direito e depende da revisão documental."));}
    }else{s.paragraphs.push("RELATÓRIO DE CONFERÊNCIA: nenhum resultado financeiro foi produzido. Não existe RMI zero, concessão ou regra vencedora neste documento.".into());}
    let mut rows=vec![];
    if let Some(o)=outcome{
        for(key,label)in[("blockers","Bloqueio"),("missing","Dado pendente"),("warnings","Aviso"),("known_model_risks","Risco conhecido")]{
            if let Some(items)=o.get(key).and_then(Value::as_array){
                let mut hidden=0usize;
                for(index,item)in items.iter().take(8).enumerate(){
                    let code=item.get("code").and_then(Value::as_str).or_else(||item.as_str());
                    let text=if key=="known_model_risks"{code.and_then(risk_label).map(str::to_owned)}
                        else if personal{item.get("message").and_then(Value::as_str).or_else(||item.as_str()).map(field).transpose()?.filter(|value|!value.is_empty())}
                        else{code.and_then(pending_label).or_else(||code.and_then(risk_label)).map(str::to_owned)};
                    match text{Some(value)=>rows.push(vec![format!("{label} {}",index+1),value]),None=>hidden+=1}
                }
                let omitted=items.len().saturating_sub(8);
                if omitted>0||hidden>0{rows.push(vec![format!("{label}: {} no total",items.len()),format!("{} ocorrência(s) não catalogada(s) ou texto livre oculto; consulte a aplicação autorizada.",omitted+hidden)]);}
            }
        }
    }
    if rows.is_empty(){rows.push(vec!["Condição".into(),"A conclusão depende de revisão documental e profissional; nenhum risco adicional foi retornado pelo motor.".into()]);}
    s.tables.push(table("O que pode mudar esta conclusão",&["Categoria","Descrição"],&[3,7],rows));
    s.tables.push(table("Dados efetivamente considerados",&["Item","Situação"],&[4,6],considered_rows(case,personal,native.is_some())?));
    s.paragraphs.push("A fonte importada não é reescrita. O motor recebe linhas manuais revisadas; a exclusão da fonte não corrige o documento nem comprova que a exclusão esteja certa.".into());
    Ok(s)
}
fn quality(case:&Calculation,personal:bool)->Result<Section,String>{
    let mut s=section("Qualidade de segurado - conferência por vínculo",false);
    let Some(cnis)=&case.cnis else{s.paragraphs.push("Sem CNIS anexado: não há vínculos de fonte para conferir nesta seção.".into());return Ok(s);};
    s.paragraphs.push("Esta seção não calcula período de graça, prorrogações, carência ou enquadramento. Ela reúne, por vínculo, a filiação informada, o primeiro e o último lançamento e as contribuições desconsideradas. Ausência de lançamento não prova ausência de filiação.".into());
    let mut rows=vec![];
    for link in &cnis.links{
        let entries=cnis.salary_entries.iter().filter(|entry|entry.link_sequence==Some(link.sequence)).collect::<Vec<_>>();
        let first=entries.iter().map(|entry|entry.competence.as_str()).min();
        let last=entries.iter().map(|entry|entry.competence.as_str()).max();
        let excluded=entries.iter().filter(|entry|entry.excluded_from_validated_domain).count();
        let mut pending=vec![];
        if link.prpps{pending.push("PRPPS");}
        if !link.indicators.is_empty(){pending.push("ind.");}
        if link.end_date.is_none(){pending.push("fim");}
        if entries.is_empty(){pending.push("sem lanç.");}
        let period=match(first,last){
            (Some(a),Some(b)) if a==b=>month(a)?,
            (Some(a),Some(b))=>format!("{} a {}",month(a)?,month(b)?),
            _=>"Sem lançamento".into()};
        let end=match link.end_date.as_deref(){None=>"Ausente".into(),Some(value) if iso_valid(value)=>date(Some(value)),Some(_)=>"Inválida".into()};
        rows.push(vec![
            format!("{:02}",link.sequence),
            if link.affiliation.is_some(){"Informada".into()}else{"Não informada".into()},
            period,entries.len().to_string(),excluded.to_string(),end,
            if pending.is_empty(){"Conferência pendente".into()}else{pending.join(", ")},
        ]);
    }
    s.tables.push(table("Dados por vínculo - sem cálculo de graça",&["Vínculo","Filiação","1º / último lançamento","Lanç.","Desc.","Fim original","Pendências"],&[1,2,3,1,1,2,3],rows));
    if personal {
        let mut affiliations=vec![];
        for link in &cnis.links {
            if let Some(value)=link.affiliation.as_deref().filter(|value|!value.trim().is_empty()) {
                affiliations.push(vec![format!("Vínculo {:02}",link.sequence),field(value)?]);
            }
        }
        if !affiliations.is_empty(){s.tables.push(table("Filiação informada na fonte",&["Vínculo","Filiação"],&[2,8],affiliations));}
    }
    s.paragraphs.push("Contribuições desconsideradas são as marcadas pela fonte como fora do domínio validado. Elas não são convertidas em carência nem excluídas silenciosamente. PRPPS = regime próprio/CTC; ind. = indicadores; fim = data final ausente; sem lanç. = vínculo sem remuneração associada. A filiação detalhada aparece na tabela seguinte quando a identificação é autorizada.".into());
    Ok(s)
}
fn is_planning(case:&Calculation)->bool{
    case.assistant.as_ref().and_then(|a|a.process_kind.as_deref())==Some("Planejamento futuro")||case.scenario.to_lowercase().contains("planejamento")
}
fn planning(case:&Calculation,personal:bool)->Result<Section,String>{
    let mut s=section("Planejamento - premissas declaradas e lacunas",false);
    s.paragraphs.push("Este documento não projeta renda futura. As premissas abaixo foram declaradas ou estão ausentes; nenhum valor de retorno, imposto ou contribuição foi inventado.".into());
    let policy=policy_value(case);let p=policy.as_ref();
    let native_options=p.and_then(|v|v.get("native_options"));
    let mut rows=vec![];
    for(label,status)in[("Contribuição futura","Não implementada"),("Frequência de contribuição","Não implementada"),("Alíquota","Não implementada"),("Expectativa de recebimento","Não implementada"),("IRRF","Não implementado"),("Retorno do investimento","Não implementado"),("Crescimento anual da RMI","Não implementado")]{rows.push(vec![label.into(),status.into()]);}
    rows.push(vec!["Expectativa de sobrevida".into(),match native_options.and_then(|v|v.get("expectativaSobrevida")).and_then(Value::as_str).filter(|v|!v.trim().is_empty()){
        Some(value) if personal=>field(value)?,Some(_)=>"Declarada; caminho de fator bloqueado".into(),None=>"Não informada".into()}]);
    rows.push(vec!["Referência de correção".into(),match native_options.and_then(|v|v.get("referenciaCorrecao")).and_then(Value::as_str).filter(|v|!v.trim().is_empty()){
        Some(value) if personal=>field(value)?,Some(_)=>"Declarada".into(),None=>"Não informada".into()}]);
    rows.push(vec!["Índice de correção".into(),if case.options.correction_index.trim().is_empty(){"Não informado".into()}else if personal{field(&case.options.correction_index)?}else{"Declarado".into()}]);
    s.tables.push(table("Premissas e lacunas do planejamento",&["Premissa","Situação"],&[4,6],rows));
    s.paragraphs.push("A ausência de um módulo de planejamento é uma limitação declarada, não um resultado zero.".into());
    Ok(s)
}
/// Apuração da qualidade de segurado por vínculo, com cálculo próprio.
/// A fonte não é alterada: as contribuições vêm do CNIS lido e as exclusões
/// vêm do livro de revisão declarado pelo operador.
/// Analise da Situacao Atual: requisitos apurados e cenario financeiro.
fn analysis_status(case:&Calculation,_options:ReportOptions,native:Option<&Value>)->Result<Section,String>{
    let mut s=section("Situacao atual - requisitos apurados",false);
    s.paragraphs.push("Apuracao sobre os dados confirmados neste cenario. A comparacao com os requisitos legais e informativa: nao constitui reconhecimento de direito.".into());
    let sex=case.sex.as_deref().unwrap_or("").to_lowercase();
    let birth=case.identity.birth_date.as_deref().unwrap_or("?");
    s.paragraphs.push(format!("Sexo: {}. Nascimento: {}. DIB de referencia: {}.",if sex.is_empty(){"nao informado"}else{sex.as_str()},birth,case.dib));
    if let Some(n)=native{
        if let Some(t)=n.pointer("/tempo/total/days_30_360").and_then(Value::as_i64){
            let years=t/360;let months=t%360/30;let days=t%30;
            s.paragraphs.push(format!("Tempo apurado: {years} ano(s), {months} mes(es) e {days} dia(s) (convencao 30/360)."));
        }
        if let Some(c)=n.pointer("/resumo/carencia_meses").and_then(Value::as_u64){
            s.paragraphs.push(format!("Carencia apurada: {c} competencia(s)."));
        }
        if let Some(sb)=n.pointer("/resumo/salario_beneficio").and_then(Value::as_str){
            s.paragraphs.push(format!("Salario de beneficio: R$ {sb}."));
        }
        if let Some(rmi)=n.pointer("/resumo/rmi"){
            if rmi.is_null(){
                s.paragraphs.push("RMI: nao apurada (requisitos insuficientes neste cenario).".into());
            }else if let Some(v)=rmi.as_str(){
                s.paragraphs.push(format!("RMI estimada: R$ {v}."));
            }
        }
        if let Some(rule)=n.pointer("/regra_escolhida").and_then(Value::as_str){
            s.paragraphs.push(format!("Regra de transicao aplicavel: {rule}."));
        }
    }else{
        s.paragraphs.push("Resultado nativo nao disponivel: dados ou politica pendentes. A analise fica restrita aos requisitos declarados.".into());
    }
    Ok(s)
}
/// Financeiro: A PAGAR (contribuicoes necessarias) vs A RECEBER (beneficio estimado).
fn analysis_financial(_case:&Calculation,_options:ReportOptions,native:Option<&Value>)->Result<Section,String>{
    let mut s=section("Cenario financeiro - a pagar e a receber",false);
    s.paragraphs.push("Comparacao informativa entre o esforco contributivo e o beneficio estimado. Valores sao simulacoes, nao calculos de debito ou credito.".into());
    if let Some(n)=native{
        let soma=n.pointer("/correcao/soma_considerada").and_then(Value::as_str).unwrap_or("?");
        let rmi=n.pointer("/resumo/rmi").and_then(Value::as_str).unwrap_or("?");
        let competencias=n.pointer("/resumo/competencias").and_then(Value::as_u64).unwrap_or(0);
        s.paragraphs.push(format!("A PAGAR (contribuicoes consideradas no PBC): R$ {soma} em {competencias} competencia(s)."));
        if rmi!="?" && rmi!="null"{
            let meses_vida=371.0;
            let total_receber: f64 = rmi.parse::<f64>().unwrap_or(0.0)*meses_vida;
            s.paragraphs.push(format!("A RECEBER (estimativa com expectativa de 371 meses, sem 13o/IRRF): R$ {total_receber:.2}."));
            let soma_val=soma.parse::<f64>().unwrap_or(0.0);
            if soma_val>0.0{
                let ratio=total_receber/soma_val;
                s.paragraphs.push(format!("Relacao beneficio/contribuicao: {ratio:.2}x (cada R$ 1,00 contribuido retorna R$ {ratio:.2} em beneficio estimado)."));
            }
        }else{
            s.paragraphs.push("A RECEBER: nao apurado (RMI indisponivel neste cenario).".into());
        }
    }else{
        s.paragraphs.push("Sem resultado nativo: o cenario financeiro requer dados e politica confirmados.".into());
    }
    Ok(s)
}/// ROI: projecao de beneficio ao longo da expectativa de vida, com 13o e IRRF oficial.
fn analysis_roi(native:Option<&Value>)->Result<Section,String>{
    let mut s=section("Retorno do investimento (ROI)",false);
    s.paragraphs.push("Projecao informativa do beneficio ao longo da expectativa de vida. Considera 13o salario (1/12 adicional por mes) e IRRF oficial pela tabela embutida (isencao de 65 anos conforme Lei 7.713/1988, art. 6o, XIV). Nao inclui reajustes anuais.".into());
    let Some(n)=native else{s.paragraphs.push("Sem resultado nativo: o ROI requer dados e politica confirmados.".into());return Ok(s);};
    let Some(rmi_str)=n.pointer("/resumo/rmi").and_then(Value::as_str) else{s.paragraphs.push("RMI indisponivel: ROI nao calculado.".into());return Ok(s);};
    let rmi: f64 = rmi_str.parse().unwrap_or(0.0);
    if rmi<=0.0{s.paragraphs.push("RMI zero ou invalida: ROI nao calculado.".into());return Ok(s);}
    let soma_str=n.pointer("/correcao/soma_considerada").and_then(Value::as_str).unwrap_or("0");
    let soma: f64 = soma_str.parse().unwrap_or(0.0);
    let meses_vida=371.0;
    let rmi_com_13o=rmi*(13.0/12.0);
    // IRRF oficial: usa a tabela embutida do rgps-core.
    let competencia_irrf="2026-01";
    let base_irrf_str=format!("{:.2}",rmi_com_13o);
    let irrf_result=rgps_core::fiscal::irrf_mensal(competencia_irrf,&base_irrf_str,0,None);
    let (irrf_com_13o,irrf_label)=match &irrf_result{
        Ok(v)=>(f64::try_from(v.irrf).unwrap_or(0.0),format!("IRRF oficial (tabela {})",v.tabela_competencia)),
        Err(_)=>{let isencao=1908.0;let aliquota=0.075;let irrf=if rmi_com_13o>isencao{(rmi_com_13o-isencao)*aliquota}else{0.0};(irrf,"IRRF estimado (fallback)".to_string())}
    };
    // IRRF com isencao 65: recalcula com isencao
    let isencao_65_str=format!("{:.2}",rmi_com_13o);
    let irrf_isento=match rgps_core::fiscal::irrf_mensal(competencia_irrf,&base_irrf_str,0,Some(&isencao_65_str)){
        Ok(v)=>f64::try_from(v.irrf).unwrap_or(0.0),
        Err(_)=>0.0,
    };
    let idade_atual_meses=n.pointer("/pcd/idade_der_meses").and_then(Value::as_i64).unwrap_or(0);
    let meses_ate_65=(65*12-idade_atual_meses).max(0) as f64;
    let meses_com_irrf=meses_ate_65.min(meses_vida);
    let meses_isentos=(meses_vida-meses_com_irrf).max(0.0);
    let total_bruto=rmi_com_13o*meses_vida;
    let total_irrf=irrf_com_13o*meses_com_irrf+irrf_isento*meses_isentos;
    let total_liquido=total_bruto-total_irrf;
    let payback_meses=if rmi_com_13o-irrf_com_13o>0.0{Some(soma/(rmi_com_13o-irrf_com_13o))}else{None};
    s.paragraphs.push(format!("RMI mensal: R$ {rmi:.2}."));
    s.paragraphs.push(format!("RMI com 13o (1/12 adicional): R$ {rmi_com_13o:.2}/mes."));
    s.paragraphs.push(format!("{irrf_label}: R$ {irrf_com_13o:.2}/mes (sem isencao 65) / R$ {irrf_isento:.2}/mes (com isencao 65)."));
    s.paragraphs.push(format!("Total bruto em {meses_vida:.0} meses (com 13o): R$ {total_bruto:.2}."));
    s.paragraphs.push(format!("Total IRRF estimado: R$ {total_irrf:.2} ({meses_com_irrf:.0} meses com aliquota, {meses_isentos:.0} meses isentos)."));
    s.paragraphs.push(format!("Total liquido estimado: R$ {total_liquido:.2}."));
    if soma>0.0{
        let roi_ratio=total_liquido/soma;
        s.paragraphs.push(format!("ROI: {roi_ratio:.2}x (cada R$ 1,00 contribuido retorna R$ {roi_ratio:.2} liquidos)."));
        if let Some(pm)=payback_meses{
            let anos=pm/12.0;
            s.paragraphs.push(format!("Payback: {pm:.0} meses ({anos:.1} anos) para recuperar as contribuicoes."));
        }
    }else{
        s.paragraphs.push("Contribuicoes nao apuradas: ROI e payback nao calculados.".into());
    }
    Ok(s)
}fn quality_calculation(case:&Calculation,personal:bool)->Result<Section,String>{
    let mut s=section("Cálculo da qualidade de segurado por vínculo",false);
    s.paragraphs.push("Apuração própria sobre as contribuições declaradas na origem, com as exclusões do livro de revisão. Regras aplicadas: período de graça de 12 meses (art. 15, II, da Lei nº 8.213/1991), prorrogado para até 24 meses quando houve mais de 120 contribuições sem interrupção (art. 15, § 1º, da mesma lei), carência de 180 contribuições mensais (art. 25, II) ou faixa da tabela do art. 142 para quem estava inscrito até 24/07/1991.".into());
    s.paragraphs.push("Limites: a carência é medida em contribuições (arts. 24 e 27, I) e o tempo intercalado em gozo de benefício não conta para carência (art. 19-C, § 1º, do Decreto nº 3.048/1999), embora conte como tempo de serviço (art. 55, II, da Lei nº 8.213/1991). A competência que aparece em mais de um vínculo é contada uma única vez, no vínculo anterior. A comparação com a carência exigida é informativa: não há direito apurado.".into());
    let Some(cnis)=&case.cnis else{s.paragraphs.push("Sem CNIS anexado: não há vínculos de origem para apurar.".into());return Ok(s);};
    let mut periods=Vec::new();
    for link in &cnis.links{
        let start=link.start_date.as_deref().and_then(|value|value.parse::<previus_core::CivilDate>().ok());
        let end=link.end_date.as_deref().and_then(|value|value.parse::<previus_core::CivilDate>().ok());
        let mut contribution_months=Vec::new();
        let mut late_months=Vec::new();
        for entry in cnis.salary_entries.iter().filter(|entry|entry.link_sequence==Some(link.sequence)){
            let competence=entry.competence.parse::<previus_core::YearMonth>().map_err(|_|"Competência de origem inválida; a apuração de qualidade foi recusada.".to_owned())?;
            if entry.indicators.iter().any(|code|code.eq_ignore_ascii_case("PREM-EXT")){ late_months.push(competence); }
            contribution_months.push(competence);
        }
        let mut disregarded=Vec::new();
        if let Some(review)=&case.contribution_review{
            for record in review.records.iter().filter(|record|record.link_sequence==Some(link.sequence)&&record.excluded){
                disregarded.push(record.competence.parse::<previus_core::YearMonth>().map_err(|_|"Competência de revisão inválida; a apuração de qualidade foi recusada.".to_owned())?);
            }
        }
        let benefit_period=link.affiliation.as_deref().map(|value|value.to_lowercase().contains("benefício")).unwrap_or(false);
        periods.push(previus_core::QualityPeriod{
            label:format!("Vínculo {:02}",link.sequence),
            start,end,
            affiliation:link.affiliation.clone(),
            affiliation_kind:previus_core::AffiliationKind::parse(link.affiliation.as_deref()),
            late_months,
            employer:link.raw_employer.clone(),
            contribution_months,disregarded_months:disregarded,
            had_benefit_episode:false,
            benefit_period,
        });
    }
    let filed_before_july_1991=cnis.links.iter().filter_map(|link|link.start_date.as_deref())
        .any(|value|value<="1991-07-24");
    let summary=previus_core::insured_quality(&previus_core::QualityInput{periods,required_carencia_months:180,implementation_year:None,filed_before_july_1991})
        .map_err(|error|error.to_string())?;
    let mut rows=vec![];
    for period in &summary.periods{
        rows.push(vec![period.label.clone(),
            period.affiliation.clone().filter(|value|!value.trim().is_empty()).unwrap_or_else(||"Não informada".into()),
            date(period.start.as_ref().map(|value|value.to_string()).as_deref()),
            match period.end{Some(value)=>date(Some(&value.to_string())),None=>"Ausente".into()},
            period.counted_months.to_string(),
            period.overlap_months.to_string(),
            period.benefit_months.to_string(),
            period.disregarded_months.to_string(),
            if personal{optional_field(period.employer.as_deref())?}else{period.label.clone()}]);
    }
    s.tables.push(table("Vínculos e contribuições apuradas",&["Vínculo","Filiação","Início","Fim","Cont.","Sob.","Ben.","Desc.","Origem do vínculo"],&[2,3,2,2,1,1,1,1,4],rows));
    let mut rows=vec![];
    for period in &summary.periods{
        rows.push(vec![period.label.clone(),
            period.affiliation.clone().filter(|value|!value.trim().is_empty()).unwrap_or_else(||"Não informada".into()),
            period.carencia_basis.to_string(),
            period.counted_months.to_string()]);
    }
    s.tables.push(table("Fundamento da contagem de carência por vínculo",&["Vínculo","Filiação","Regra aplicada","Meses contados"],&[2,3,7,2],rows));
    let mut rows=vec![];
    for period in &summary.periods{
        let payments=match(period.first_payment,period.last_payment){
            (Some(first),Some(last)) if first==last=>first.to_string(),
            (Some(first),Some(last))=>format!("{first} a {last}"),
            _=>"Sem lançamento apurado".into()};
        let grace_base=match period.grace_base_until{Some(value)=>date(Some(&value.to_string())),None=>"Não apurável".into()};
        let grace_vigente=match period.grace_until{Some(value)=>date(Some(&value.to_string())),None=>"Não apurável".into()};
        let desconsideradas=if period.disregarded_detail.is_empty(){"Nenhuma".to_string()}
            else{
                let mut por_motivo:Vec<(String,usize)>=Vec::new();
                for(_,reason) in &period.disregarded_detail{
                    let label=reason.label().to_string();
                    match por_motivo.iter_mut().find(|(name,_)|*name==label){Some((_,count))=>*count+=1,None=>por_motivo.push((label,1))}
                }
                let resumo=por_motivo.iter().map(|(name,count)|format!("{count} por {name}")).collect::<Vec<_>>().join("; ");
                let meses:Vec<String>=period.disregarded_detail.iter().take(6).map(|(month,reason)|format!("{month} ({})",reason.label())).collect();
                let resto=period.disregarded_detail.len().saturating_sub(meses.len());
                format!("{resumo}. {}{}",meses.join("; "),if resto>0{format!("; e mais {resto}")}else{String::new()})
            };
        let mut notes=period.notes.clone();
        if !period.disregarded_detail.is_empty(){notes.push(format!("Contribuições não contadas: {desconsideradas}."));}
        rows.push(vec![period.label.clone(),payments,grace_base,grace_vigente,
            if period.grace_prorogued{"Prorrogada: mais de 120 contribuições".to_string()}else{"Não aplicável".to_string()},
            if notes.is_empty(){"Sem observação".to_string()}else{field(&notes.join(" "))?}]);
    }
    s.tables.push(table("Período de graça por vínculo",&["Vínculo","1º / último pagamento","Graça base","Graça vigente","Prorrogação","Observações"],&[2,3,2,2,3,6],rows));
    let rows=vec![vec!["Meses de contribuição contados".into(),summary.total_counted_months.to_string()],
        vec!["Período coberto pela série de competências (meses, 30/360)".into(),summary.total_contribution_months_30_360.to_string()],
        vec!["Meses não contados por competência em outro vínculo".into(),summary.total_overlap_months.to_string()],
        vec!["Meses não contados por período de gozo de benefício".into(),summary.total_benefit_months.to_string()],
        vec!["Meses desconsiderados pelo operador".into(),summary.total_disregarded_months.to_string()],
        vec!["Carência exigida".into(),summary.required_carencia_months.to_string()],
        vec!["Fundamento da carência exigida".into(),field(&summary.required_carencia_basis)?],
        vec!["Comparação informativa".into(),if summary.carencia_observed{"Meses contados alcançam a carência exigida; a carência legal e o direito a benefício exigem conferência normativa".into()}else{"Meses contados abaixo da carência exigida; não há direito apurado".into()}]];
    s.tables.push(table("Totais apurados",&["Medida","Resultado"],&[5,7],rows));
    let mut notes=summary.notes.clone();
    notes.push("Cálculo próprio, não reprodução de fórmula alheia. Não certifica enquadramento, carência legal, direito a benefício ou elegibilidade.".into());
    s.paragraphs.push(notes.join(" "));
    Ok(s)
}
fn summary(case:&Calculation, options:ReportOptions, native:Option<&Value>) -> Result<Section,String> {
    let mut s=section("Síntese, situação e alcance",false);
    s.paragraphs.push("Esta seção registra os dados considerados, a origem das informações e os parâmetros declarados. Confira o resultado junto da memória de cálculo e das pendências documentais.".into());
    s.paragraphs.push("Legenda: FONTE = dados importados, sem revisão automática; DECLARADO = informado pelo operador; PENDENTE = exige conferência; EXPERIMENTAL = cálculo computacional, sem certificação jurídica.".into());
    s.paragraphs.push(if options.include_personal {"Relatório confidencial com identificação autorizada; revisar antes de compartilhar."}else{"Identificação direta e textos livres ocultos. Datas, valores e vínculos técnicos ainda permitem correlação: este documento não é garantia de anonimização nem de ausência de vazamento."}.into());
    let mut rows=vec![vec!["DIB / referência declarada".into(),date(Some(&case.dib))],vec!["Natureza da referência".into(),"Hipótese do cenário; não data concedida pelo INSS".into()],
        vec!["Sexo declarado".into(),enum_text(case.sex.as_deref(),&["masculino","feminino","Masculino","Feminino"])],
        vec!["Deficiência declarada".into(),declared_disability(case.disability.as_deref())],
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
    s.tables.push(table("Política declarada: não substitui os parâmetros efetivamente usados",&["Campo","Declaração"],&[5,5],rows));
    Ok(s)
}
fn periods(case:&Calculation,personal:bool,native:Option<&Value>)->Result<Section,String>{
    let mut s=section("Períodos de contribuição",true);
    s.paragraphs.push("As datas abaixo são as reportadas na fonte. Fim ausente permanece ausente; candidato de último salário não é fim de vínculo. PRPPS/RPPS não é convertido para RGPS por declaração genérica.".into());
    if let Some(cnis)=&case.cnis {
        let mut rows=vec![];
        for l in &cnis.links {rows.push(vec![l.sequence.to_string(),if personal{optional_field(l.raw_employer.as_deref())?}else{format!("Vínculo {:02}",l.sequence)},
            date(l.start_date.as_deref()),date(l.end_date.as_deref()),if l.prpps{"PRPPS pendente"}else{"Fonte não revisada"}.into(),
            indicators(&l.indicators,personal)?,source(&l.source)]);}
        s.tables.push(table("Todos os vínculos originais",&["Nº","Vínculo / empregador","Início","Fim original","Situação","Indicadores","Fonte"],&[1,4,2,2,3,3,2],rows));
        let mut rows=vec![];for l in &cnis.links {if let Some(candidate)=&l.inferred_last_salary_end{rows.push(vec![l.sequence.to_string(),month(&candidate.competence)?,date(Some(&candidate.date_candidate)),"Candidato não confirmado; não usado como fim original".into(),source(&candidate.source)]);}}
        if !rows.is_empty(){s.tables.push(table("Candidatos de fim: separados da informação original",&["Vínculo","Última competência","Data candidata","Alcance","Fonte"],&[1,2,2,5,2],rows));}
        let rows=cnis.benefits.iter().map(|b|vec![b.sequence.to_string(),b.species.map(|v|v.to_string()).unwrap_or("Não informada".into()),match b.status{BenefitStatus::Denied=>"Indeferido",BenefitStatus::Unknown=>"Situação desconhecida"}.into(),"Não prova recebimento atual".into(),source(&b.source)]).collect();
        if !cnis.benefits.is_empty(){s.tables.push(table("Eventos de benefício: não são vínculos de emprego",&["Nº","Espécie","Situação na fonte","Alcance","Fonte"],&[1,1,3,4,2],rows));}
    }
    let mut rows=vec![];for(i,l)in case.manual_links.iter().enumerate(){rows.push(vec![(i+1).to_string(),if personal{optional_field(Some(&l.employer))?}else{format!("Período manual {:02}",i+1)},date(Some(&l.start_date)),date(Some(&l.end_date)),enum_text(Some(&l.regime),&["RGPS","RPPS","PRPPS","rgps","rpps"]),if l.source_review.as_ref().is_some_and(|r|r.excluded){"Excluído da hipótese".into()}else{bool_text(l.confirmed)},indicators(&if l.indicators.is_empty(){vec![]}else{vec![l.indicators.clone()]},personal)?]);}
    s.tables.push(table("Declarações manuais: mantidas separadas da fonte",&["Nº","Vínculo","Início","Fim","Regime","Confirmado?","Indicadores"],&[1,4,2,2,2,2,3],rows));
    let mut audit=vec![];
    for link in &case.manual_links {if let Some(review)=&link.source_review{
        for (index,change) in review.history.iter().enumerate(){audit.push(vec![review.sequence.to_string(),(index+1).to_string(),change.at.clone(),if review.excluded{"Excluído da hipótese".into()}else{"Ajuste declarado".into()},if personal{field(&change.reason)?}else{"Justificativa ocultada".into()}]);}
    }}
    if !audit.is_empty(){s.tables.push(table("Revisão de vínculos - origem preservada",&["Origem #","Operação","Data registrada","Estado","Motivo"],&[1,1,3,3,5],audit));}
    if let Some(t)=native.and_then(|v|v.get("tempo")){if !t.is_null(){
        let mut rows=vec![];for(key,label)in[("total_dias","Dias do motor"),("total_dias_efetivos","Dias efetivos do motor"),("carencia_meses","Carência do módulo de tempo")]{rows.push(vec![label.into(),numeric(t.get(key),false)?]);}
        for(key,label)in[("ate_ec20","Marco EC20, dias"),("ate_ec103","Marco EC103, dias"),("apos_ec103","Após EC103, dias")]{rows.push(vec![label.into(),numeric(t.get("marcos").and_then(|v|v.get(key)),false)?]);}
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
                match e.amount_kind{AmountKind::Remuneration=>"Remuneração",AmountKind::ContributionColumnUnresolved=>"Coluna não resolvida",AmountKind::ConsolidatedAnnualValue=>"Valor consolidado por ano civil"}.into(),indicators(&e.indicators,personal)?,source(&e.source)]);
        }
        let mut grouped=vec![];for(m,(sum,count,excluded,pages,flags))in groups{grouped.push(vec![month(m)?,count.to_string(),cents(sum),if count>1{"Múltiplos lançamentos"}else{"Um lançamento"}.into(),if excluded{"Fora do domínio validado"}else{"Fonte não revisada"}.into(),indicators(&flags.into_iter().collect::<Vec<_>>(),personal)?,pages.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")]);}
        s.tables.push(table("Todas as competências distintas",&["Competência","Linhas","Soma nominal (u.f.)","Conferência","Domínio","Indicadores","Páginas"],&[2,1,3,3,3,3,1],grouped));
        detail.tables.push(table("Todos os lançamentos importados",&["Nº","Competência","Vínculo","Valor nominal (u.f.)","Natureza","Indicadores","Fonte"],&[1,2,1,3,3,3,2],rows));
    }else{s.paragraphs.push("Não há remunerações importadas de CNIS neste cenário.".into());}
    let mut rows=vec![];for(i,m)in case.manual_salaries.iter().enumerate(){rows.push(vec![(i+1).to_string(),month(&m.competence)?,manual_amount(&m.amount)?,bool_text(m.confirmed),indicators(&if m.indicators.trim().is_empty(){vec![]}else{vec![m.indicators.clone()]},personal)?]);}
    detail.tables.push(table("Remunerações manuais: moeda depende da política declarada",&["Nº","Competência","Valor informado","Confirmada?","Indicadores"],&[1,2,3,2,4],rows));
    Ok(vec![s,detail])
}
fn pending_label(code:&str)->Option<&'static str>{
    match code {
        "currency_units_unverified"=>Some("Unidades monetárias nominais da fonte não conferidas/conversões históricas não aplicadas."),
        "source_review_required"|"review_not_confirmed"=>Some("Revisão documental por registro ainda exigida."),
        "prpps_unvalidated"=>Some("Vínculos PRPPS/RPPS não validados como RGPS; CTC não presumida."),
        "source_end_missing"|"missing_or_invalid_date"=>Some("Data ausente/inválida exige confirmação documental."),
        "pcd_lc142_not_implemented"=>Some("Cálculo PcD LC 142/2013 ainda não implementado; sem RMI presumida."),
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
fn risk_label(code:&str)->Option<&'static str>{
    match code {
        "unvalidated_native_fator_uses_sex_specific_rate_and_floor_one"=>Some("Fator previdenciário nativo não validado; pode não representar a regra aplicável."),
        "transitions_use_salary_month_count_not_full_confirmed_periods"=>Some("Transições usam contagem de competências, não todos os períodos confirmados."),
        "reform_month_not_split_at_2019_11_13"=>Some("O mês da reforma não é dividido em 13/11/2019."),
        "points_truncate_age_and_time_to_whole_years"=>Some("Regra de pontos trunca idade e tempo para anos inteiros."),
        "grace_and_late_payments_model_not_validated_and_adapter_blocks_them"=>Some("Período de graça e recolhimentos em atraso não são validados pelo adaptador."),
        "pre_reform_rmi_not_recalculated"=>Some("RMI de legislação anterior não é recalculada por este caminho."),
        "discard_and_divisor_are_operator_options_not_date_selected_law"=>Some("Descarte e divisor mínimo são opções do operador, não regra selecionada pela data."),
        "native_corrected_pbc_does_not_apply_minimum_divisor_option"=>Some("PBC corrigido não aplica a opção de divisor mínimo."),
        "time_markers_may_double_count_overlapping_links"=>Some("Marcos de tempo podem contar períodos sobrepostos."),
        "tables_are_owned_source_snapshot_not_freshly_verified_official_values"=>Some("Tabelas são snapshot da fonte observada, não valores oficiais reverificados."),
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
        for (key,label) in [("blockers","Bloqueio"),("missing","Dado pendente"),("warnings","Aviso"),("known_model_risks","Risco conhecido")] {
            if let Some(vs)=o.get(key).and_then(Value::as_array) {
                let mut hidden=0;
                for (i,v) in vs.iter().enumerate() {
                    let code=v.get("code").and_then(Value::as_str).or_else(||v.as_str());
                    if !personal && code.and_then(pending_label).is_none() { hidden+=1; continue; }
                    let message=if personal {v.get("message").and_then(Value::as_str).or_else(||v.as_str()).map(field).transpose()?.unwrap_or_else(||"Estrutura de aviso não reconhecida; conteúdo não publicado.".into())}
                        else {code.and_then(pending_label).unwrap().into()};
                    rows.push(vec![format!("{label} {}",i+1),message]);
                }
                if hidden>0 { rows.push(vec![format!("{label}: {hidden} ocorrência(s)"),"Textos livres ocultos neste modo. Consulte cada pendência na aplicação autorizada.".into()]); }
            }
        }
        s.tables.push(table("Guardas, avisos e riscos do resultado explicitamente fornecido",&["Categoria","Ressalva"],&[3,7],rows));
        if let Some(ns)=o.pointer("/data/native_result/avisos").and_then(Value::as_array) {
            let rows=if personal { ns.iter().enumerate().map(|(i,v)|Ok(vec![format!("Aviso nativo {}",i+1),optional_field(v.as_str())?])).collect::<Result<Vec<_>,String>>()? }
                else if ns.is_empty() {vec![]}
                else {vec![vec![format!("Avisos do motor: {}",ns.len()),"Textos livres ocultos neste modo. Resultado experimental; confira todos os avisos na aplicação.".into()]]};
            s.tables.push(table("Avisos da execução nativa",&["Registro","Ressalva"],&[3,7],rows));
        }
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
    if let Some(items)=n.get("memoria").and_then(Value::as_array){for(i,v)in items.iter().enumerate(){let kind=v.get("tipo").and_then(Value::as_str);let known=match kind{Some("media")=>Some("Média / salário de benefício"),Some("coeficiente")=>Some("Coeficiente computacional"),Some("rmi")=>Some("Salário de benefício × coeficiente"),Some("limite")=>Some("Aplicação de piso ou teto"),Some("especie")=>Some("Espécie e regra aplicada"),Some("requisito")=>Some("Requisito da espécie"),_=>None};let value=v.get("valor").and_then(Value::as_str);let number=match kind{Some("especie")|Some("requisito")=>value.unwrap_or("Não produzido").to_string(),_=>match value{Some(v)=>if let Some(v)=v.strip_suffix('%'){format!("{}%",decimal(v,false)?)}else{decimal(v,true)?},None=>"Não produzido".into()}};
        rows.push(vec![(i+1).to_string(),known.unwrap_or("Etapa não catalogada").into(),if personal||matches!(kind,Some("especie")|Some("requisito")){optional_field(v.get("descricao").and_then(Value::as_str))?}else{known.unwrap_or("Descrição livre oculta").into()},number]);}}
    memory.tables.push(table("Todas as etapas numéricas retornadas",&["Nº","Etapa","Descrição","Valor fornecido"],&[1,3,6,3],rows));
    if let Some(c)=n.get("carencia"){let mut rows=vec![];for(key,label)in[("competencias_salario","Competências de remuneração"),("competencias_vinculo","Competências de vínculos"),("competencias_graca","Competências de graça"),("recolhimentos_atraso","Recolhimentos em atraso"),("carencia_meses","União de competências, meses")]{rows.push(vec![label.into(),numeric(c.get(key),false)?]);}memory.tables.push(table("Decomposição da carência computacional",&["Origem","Quantidade fornecida"],&[6,4],rows));}
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
        correction.tables.push(table("Referências declaradas pelo resultado: não certificadas pelo renderer",&["ID","Descrição"],&[1,9],rows));
    }else{correction.paragraphs.push("Resultado não contém correção parcela a parcela. Sem série de índices fabricada; média nominal somente se explicitamente retornada pelo motor.".into());}
    for(key,label)in[("parametros","Parâmetros"),("resumo","Resumo"),("tempo","Tempo"),("carencia","Carência"),("correcao","Correção"),("memoria","Memória"),("transicoes","Transições"),("especie","Espécie"),("beneficio","Requisitos")]{if let Some(v)=n.get(key){let known:&[&str]=match key{
        "resumo"=>&["competencias","carencia_meses","soma_salarios","salario_beneficio","coeficiente","rmi_bruta","rmi","limite_aplicado","teto","piso"],
        "parametros"=>&["divisor","divisor_minimo_aplicado","anos","meses_restantes","anos_base","coeficiente","correcao_ativa","referencia_correcao","descarte_aplicado"],
        "tempo"=>&["vinculos","total_dias","total_dias_efetivos","total","carencia_meses","marcos","avisos"],
        "carencia"=>&["competencias_salario","competencias_vinculo","competencias_graca","recolhimentos_atraso","carencia_meses","competencias_extras","avisos"],
        "correcao"=>&["referencia","pbc","consideradas","descartadas","parcelas","parcelas_descartadas","soma_considerada","media","salario_beneficio","descarte_aplicado","fontes_tetos","fontes_inpc"],
        "transicoes"=>&["referencia","sexo","idade_meses","idade_anos","tempo_total_meses","tempo_total_anos","tempo_ate_reforma_meses","regras","regra_escolhida","numero_regra_transicao","rmi_escolhida","observacoes"],
        "especie"=>&["codigo","nome","regra_escolhida","elegivel"],
        "beneficio"=>&["especie","nome","coeficiente","regra_escolhida","regras","base_legal","limite_media_12","observacoes"],_=>&[]};
        if let Some(object)=v.as_object(){let count=object.keys().filter(|k|!known.contains(&k.as_str())).count();if count>0{memory.paragraphs.push(format!("{label}: {count} campo(s) fora do contrato documental não publicado(s), para evitar expor dados arbitrários do resultado."));}}
    }}
    Ok(vec![sum,memory,correction])
}
fn fabrica_section(fabrica:&Value,case:&Calculation)->Result<Section,String>{
    let mut s=section("Conferência na Fábrica de Cálculos/TRF3",true);
    s.paragraphs.push("Consulta externa opcional feita pelo operador. O resultado do provedor depende dos dados enviados e das regras dele; não é certificação de direito, concessão ou perícia. O motor local continua experimental e separado.".into());
    let provider=fabrica.get("provider").unwrap_or(&Value::Null);
    let best=provider.get("beneficios").and_then(Value::as_array).and_then(|items|items.first());
    let mut rows=vec![
        vec!["Fonte".into(),optional_field(fabrica.get("source_url").and_then(Value::as_str))?],
        vec!["Consultado em".into(),optional_field(fabrica.get("retrieved_at").and_then(Value::as_str))?],
        vec!["RMI do provedor".into(),numeric(provider.get("maiorRmi"),true)?],
        vec!["Regra do provedor".into(),optional_field(best.and_then(|b|b.get("fundamento")).and_then(Value::as_str))?],
        vec!["Salário de benefício".into(),numeric(best.and_then(|b|b.pointer("/dadosRmi/salarioBeneficio")),true)?],
        vec!["Coeficiente".into(),best.and_then(|b|b.pointer("/dadosRmi/coeficiente")).and_then(Value::as_f64).map(|v|format!("{:.2}%",v*100.0)).unwrap_or_else(||"Não apurado".into())],
        vec!["Divisor / competências".into(),format!("{} / {}",numeric(best.and_then(|b|b.pointer("/dadosRmi/divisor")),false)?,numeric(best.and_then(|b|b.pointer("/dadosRmi/totalCompetenciasConsideradas")),false)?)],
        vec!["RMI local no relatório".into(),case.result.as_ref().and_then(|r|r.pointer("/data/native_result/transicoes/rmi_escolhida").or_else(||r.pointer("/data/native_result/resumo/rmi"))).map(|v|numeric(Some(v),true)).transpose()?.unwrap_or_else(||"Não apurada".into())],
        vec!["Hash da resposta".into(),fabrica.get("response_sha256").and_then(Value::as_str).map(|v|v.chars().take(32).collect::<String>()+"…").unwrap_or_else(||"Não registrado".into())],
    ];
    if let Some(items)=best.and_then(|b|b.get("requisitos")).and_then(Value::as_array){
        for item in items{
            let name=item.get("rotulo").or_else(||item.get("nome")).and_then(Value::as_str).unwrap_or("Requisito");
            let text=optional_field(item.get("porExtenso").and_then(Value::as_str))?;
            let text=if text.is_empty(){"Resultado não detalhado".to_string()}else{text};
            let status=match item.get("cumprido").and_then(Value::as_bool){Some(true)=>"Cumprido",Some(false)=>"Não cumprido",None=>"—"};
            rows.push(vec![format!("Requisito · {name}"),format!("{text} ({status})")]);
        }
    }
    s.tables.push(table("Conferência externa e comparação com o resultado local",&["Item","Valor fornecido"],&[5,5],rows));
    if let Some(conclusao)=provider.get("conclusao"){
        let completo=conclusao.get("completo").and_then(Value::as_bool);
        if completo==Some(false){s.paragraphs.push(format!("O provedor interrompeu o cálculo{}.",conclusao.get("motivoInterrupcao").and_then(Value::as_str).map(|v|format!(": {v}")).unwrap_or_default()));}
    }
    s.paragraphs.push("A conferência não substitui a revisão documental, a perícia, o nexo, a DII ou a conferência de CTC. O hash registra a resposta recebida, não uma assinatura do provedor.".into());
    Ok(s)
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
            if !rows.is_empty(){s.tables.push(table(&format!("Regra {}: requisitos e faltas",i+1),&["Item","Detalhe fornecido"],&[4,6],rows));}
        }
    }
    s.paragraphs.push("*Critério atingido = flag elegivel do motor, não concessão nem conclusão jurídica. Vencedor do comparador pode diferir do coeficiente/resumo do motor; ambos são preservados sem forçar igualdade.".into());Ok(s)
}
fn contribution_ledger(case:&Calculation, personal:bool)->Result<Section,String>{
    let mut s=section("Livro de revisão de contribuições",false);
    let Some(review)=&case.contribution_review else{return Ok(s)};
    review.validate(case)?;
    s.paragraphs.push(if review.applied{"Projeção aplicada aos salários manuais. Não confirma regime, direito ou exclusão da fonte."}else{"Revisão não aplicada. Ajustes ainda não compõem os salários do motor."}.into());
    s.paragraphs.push("A fonte CNIS permanece separada e intacta. Somas são nominais; não há conversão automática de moeda ou validação jurídica. Histórico local não é assinatura digital.".into());
    let mut rows=vec![];
    for record in &review.records{
        let original=if record.source_indices.is_empty(){"Manual".into()}else{
            let mut total=0i64;
            for index in &record.source_indices{
                let entry=case.cnis.as_ref().and_then(|c|c.salary_entries.iter().find(|e|e.source.index==*index)).ok_or("Origem de contribuição ausente no relatório.")?;
                total=total.checked_add(entry.amount_centavos).ok_or("Soma original excede limite monetário.")?;
            }
            manual_amount(&format!("{}.{:02}",total/100,total%100))?
        };
        rows.push(vec![month(&record.competence)?,if record.source_indices.is_empty(){"Manual".into()}else{record.source_indices.iter().map(|i|i.to_string()).collect::<Vec<_>>().join(", ")},original,manual_amount(&record.amount)?,record.currency.clone(),if record.excluded{"Excluído da hipótese".into()}else if record.confirmed{"Conferência declarada".into()}else{"Pendente".into()},if personal{field(&record.evidence)?}else{"Justificativa ocultada".into()}]);
    }
    s.tables.push(table("Origem e ajustes documentados",&["Competência","Origem #","Original","Ajustado","Moeda","Situação","Justificativa"],&[12,8,12,12,10,20,26],rows));
    let history=review.history.iter().map(|entry|Ok(vec![entry.sequence.to_string(),entry.action.clone(),entry.at.clone(),if personal{field(&entry.reason)?}else{"Motivo ocultado".into()}])).collect::<Result<Vec<_>,String>>()?;
    if !history.is_empty(){s.tables.push(table("Operações de revisão",&["Operação","Ação","Data registrada","Motivo"],&[10,15,30,45],history));}
    Ok(s)
}

fn pcd_report(case:&Calculation,w:&Workspace,o:ReportOptions,n:&Value)->Result<Report,String>{
 let grade=enum_text(case.disability.as_deref(),&["Grave","Moderada","Leve","grave","moderada","leve"]);
 let pcd=&n["pcd"];let mut sections=vec![];
 let ledger=||->Result<Section,String>{
  let mut s=section("Salários de contribuição - cálculo da média",false);
  let items=n["correcao"]["parcelas"].as_array().ok_or("Resultado PcD sem parcelas de correção.")?;
  if items.len()>2400{return Err("Relatório PcD excede o limite de parcelas.".into());}
  let rows=items.iter().enumerate().map(|(i,r)|Ok(vec![(i+1).to_string(),month(r["competencia"].as_str().ok_or("Parcela sem competência.")?)?,numeric(r.get("salario"),true)?,numeric(r.get("teto"),true)?,numeric(r.get("considerado"),true)?,numeric(r.get("indice"),false)?,numeric(r.get("corrigido"),true)?,if r["descartada"]==true{"D"}else{""}.into(),match r["origem"].as_str(){Some("imported")=>"CNIS",Some("reviewed_overlay")=>"Ajuste",Some("declared_missing_minimum")=>"Mínimo*",Some("declared_projection")=>"Projeção*",_=>"Pendente"}.into()])).collect::<Result<Vec<_>,String>>()?;
  s.tables.push(table("PBC completo, em ordem cronológica",&["Nº","Data","Salário nominal","Teto da época","Salário considerado","Índice","Salário corrigido","Uso","Origem"],&[5,10,14,13,14,13,14,3,10],rows));
  s.paragraphs.push("D = descartado da média. * Preenchimento/projeção por premissa declarada, não remuneração extraída. O perfil comparativo usa a parte inteira de 80% da quantidade de parcelas. Fatores exibidos com 6 casas truncadas; salários calculados com o fator integral, antes do arredondamento em centavos. Não houve conversão monetária ou pagamento presumido.".into());Ok(s)
 };
 let totals=||->Result<Section,String>{let mut s=section("Cálculo da renda mensal inicial - LC 142/2013",false);let c=&n["correcao"];let r=&n["resumo"];s.tables.push(table("Memória da média e do coeficiente",&["Apuração","Resultado"],&[7,4],vec![vec!["Parcelas do PBC".into(),numeric(c.get("pbc"),false)?],vec!["Salários utilizados (80% maiores)".into(),numeric(c.get("consideradas"),false)?],vec!["Salários descartados".into(),numeric(c.get("descartadas"),false)?],vec!["Soma dos salários utilizados".into(),numeric(c.get("soma_considerada"),true)?],vec!["Média dos salários corrigidos".into(),numeric(c.get("media"),true)?],vec!["Coeficiente do cenário (%)".into(),numeric(r.get("coeficiente"),false)?],vec!["Renda mensal inicial - simulação".into(),numeric(r.get("rmi"),true)?]]));
 s.paragraphs.push(if r["rmi"].is_null(){"Requisito de tempo/carência não atingido no cenário; não há RMI de benefício produzida."}else{"Valor simulado com premissas declaradas. Não é concessão, laudo ou validação de direito."}.into());Ok(s)};
 let time=||->Result<Section,String>{let mut s=section("Relatório do tempo de contribuição - LC 142",false);let t=&n["tempo"];let fmt=|v:&Value|format!("{} anos, {} meses e {} dias",v["years"].as_i64().unwrap_or(0),v["months"].as_i64().unwrap_or(0),v["days"].as_i64().unwrap_or(0));s.tables.push(table("Apuração do cenário",&["Medida","Apurado"],&[6,5],vec![vec!["Data-base dos dados conhecidos".into(),date(pcd["calculation_date"].as_str())],vec!["Tempo até a data-base".into(),fmt(&t["sem_projecao"])],vec!["DIB da hipótese".into(),date(Some(&case.dib))],vec!["Tempo total na DIB, incluindo projeção".into(),fmt(&t["total"])],vec!["Tempo exigido no cenário (anos)".into(),numeric(pcd.get("anos_exigidos"),false)?],vec!["Carência no cenário, incluindo projeção (meses)".into(),numeric(n["resumo"].get("carencia_meses"),false)?],vec!["Requisito atingido nesta simulação?".into(),bool_text(pcd["tempo_atingido_no_cenario"]==true)]]));
 let rows=t["vinculos"].as_array().into_iter().flatten().map(|v|vec![date(v["inicio"].as_str()),date(v["fim"].as_str()),v["dias_30_360"].to_string(),"Intervalo unido; inclui continuidade/projeção declarada quando houver".into()]).collect();s.tables.push(table("Períodos considerados, sem duplicar concomitâncias",&["Início","Fim declarado","Dias 30/360","Alcance"],&[2,2,2,6],rows));s.paragraphs.push("Convenção declarada: meses de 30 dias, dias inclusivos e união das sobreposições. A data projetada não substitui o fim ausente no documento de origem.".into());Ok(s)};
 let assumptions=||->Result<Section,String>{let mut s=section("Premissas, fontes e limites",false);s.tables.push(table("Separação entre fonte e hipótese",&["Item","Declaração"],&[5,7],vec![vec!["Grau de deficiência constante".into(),grade.clone()],vec!["Competências preenchidas pelo mínimo".into(),numeric(pcd.get("filled_minimum_count"),false)?],vec!["Competências projetadas".into(),numeric(pcd.get("projection_count"),false)?],vec!["Referência de correção conhecida".into(),month(n["correcao"]["referencia"].as_str().ok_or("Referência ausente.")?)?],vec!["Fonte pública".into(),"Fábrica de Cálculos/TRF3 - índices e limites. Valores obtidos por GET, sem enviar CNIS.".into()],vec!["Hash do snapshot".into(),optional_field(pcd["snapshot_sha256"].as_str())?],vec!["Conferência da origem".into(),if o.include_personal{optional_field(pcd["review_evidence"].as_str())?}else{"Referência livre ocultada".into()}],vec!["Deficiência e período".into(),if o.include_personal{optional_field(pcd["disability_evidence"].as_str())?}else{"Referência livre ocultada".into()}]]));s.paragraphs.push("Premissas não são fatos extraídos: meses sem salário, continuidade e renda futura dependem da declaração do operador. Os limites futuros são congelados na data-base e não apresentados como valores oficiais futuros. Esta versão não converte graus mistos, atividade especial ou CTC automaticamente e não aplica fator previdenciário.".into());Ok(s)};
 match o.kind{
  ReportKind::Memory=>{sections.push(ledger()?);sections.push(totals()?);sections.push(assumptions()?);},
  ReportKind::Salaries=>{sections.push(ledger()?);sections.push(totals()?);sections.push(assumptions()?);},
  ReportKind::Periods=>{sections.push(time()?);sections.push(assumptions()?);},
  ReportKind::Summary=>{sections.push(totals()?);sections.push(time()?);sections.push(assumptions()?);},
  ReportKind::Detailed=>{sections.push(totals()?);sections.push(time()?);sections.push(ledger()?);sections.push(assumptions()?);},
  ReportKind::Transitions=>{let mut s=section("Comparação de cenários",false);s.paragraphs.push("Este documento calcula somente o grau constante declarado pela LC 142. Não foi executada comparação automática com todas as regras da EC 103, direito adquirido, fator ou aposentadoria por idade. Nenhum vencedor foi presumido.".into());sections.push(s);sections.push(time()?);sections.push(assumptions()?);},
  ReportKind::Indicators=>{sections.push(assumptions()?);sections.push(pending(case,o.include_personal,None)?);},
  ReportKind::Quality=>{sections.push(assumptions()?);sections.push(pending(case,o.include_personal,None)?);},
        ReportKind::Analysis=>{sections.push(assumptions()?);sections.push(pending(case,o.include_personal,None)?);},
        ReportKind::Roi=>{sections.push(assumptions()?);sections.push(pending(case,o.include_personal,None)?);},
        ReportKind::Regras=>{sections.push(assumptions()?);sections.push(pending(case,o.include_personal,None)?);},
 }
 let office=if o.include_personal&&!w.settings.office_name.trim().is_empty(){field(&w.settings.office_name)?}else{"previdenciar.io - cálculo previdenciário".into()};
 if office.chars().count()>120{return Err("Cabeçalho excede o espaço reservado; nenhum texto foi cortado.".into());}
 Ok(Report{title:format!("{} - LC 142/2013 ({grade})",document_title(case,o)?),status:"SIMULAÇÃO - PREMISSAS DECLARADAS - NÃO É CONCESSÃO".into(),office,footer:"Origem preservada. Projeções não são pagamentos. Revisão profissional necessária.".into(),personal:o.include_personal,identification:[format!("Nome do segurado: {}",if o.include_personal{optional_field(case.identity.full_name.as_deref())?}else{"Identificação oculta".into()}),format!("CPF: {}    Nascimento: {}",if o.include_personal{optional_field(case.identity.cpf.as_deref())?}else{"Oculto".into()},date(case.identity.birth_date.as_deref())),format!("DIB: {}    Data-base: {}    Grau: {}",date(Some(&case.dib)),date(pcd["calculation_date"].as_str()),grade)],highlight:if matches!(o.kind,ReportKind::Detailed|ReportKind::Summary|ReportKind::Memory){
    n.pointer("/resumo/rmi").filter(|v|!v.is_null()).map(|v|numeric(Some(v),true).map(|amount|format!("R$ {amount}"))).transpose()?
 }else{None},sections})
}

/// Title chosen by the operator for the document. Empty keeps the report
/// type's own title. The name is never used to infer data or entitlement.
fn document_title(case:&Calculation,options:ReportOptions)->Result<String,String>{
    let custom=case.options.report_title.trim();
    if custom.is_empty(){return Ok(options.kind.title().into());}
    if custom.chars().count()>120{return Err("Título do relatório excede 120 caracteres; nenhum texto foi cortado.".into());}
    Ok(field(custom)?)
}
pub(crate) fn build(case:&Calculation,workspace:&Workspace,options:ReportOptions,outcome:Option<&Value>)->Result<Report,String>{
    previus_workspace::link_review::validate_source_links(case)?;
    if let Some(v)=outcome{json_budget(v)?;}
    if options.include_personal { field(&case.name)?; }
    let ledger_count=case.contribution_review.as_ref().map_or(0,|review|review.records.len()+review.history.len());
    let input_count=ledger_count.checked_add(case.manual_links.len()).ok_or("Quantidade de registros excede limite.")?.checked_add(case.manual_salaries.len()).and_then(|v|v.checked_add(case.cnis.as_ref().map_or(0,|c|c.links.len()+c.salary_entries.len()+c.findings.len()+c.warnings.len()))).ok_or("Quantidade de registros excede limite.")?;
    if input_count>MAX_ROWS{return Err("Entrada excede 10.000 registros; nenhum registro foi omitido para caber no relatório.".into());}
    let native=outcome.filter(|v|v.get("status").and_then(Value::as_str)==Some("experimental")).and_then(|v|v.pointer("/data/native_result")).filter(|v|v.is_object());
    let mut report=Report{title:document_title(case,options)?,status:if native.is_some(){"RESULTADO EXPERIMENTAL / REGRAS NÃO CERTIFICADAS"}else{"RELATÓRIO DE CONFERÊNCIA / DADOS PENDENTES"}.into(),
        office:if options.include_personal{let office=workspace.settings.office_name.trim();if office.is_empty(){"previdenciar.io • RELATÓRIOS".into()}else{field(office)?}}else{"Escritório previdenciário".into()},
        footer:if options.include_personal{let f=workspace.settings.report_footer.trim();if f.is_empty(){"Uso confidencial. Revisão documental e normativa necessária.".into()}else{field(f)?}}else{"Identificação oculta. Conferir dados, documentos e parâmetros.".into()},personal:options.include_personal,identification: [
            format!("Nome do segurado: {}", if options.include_personal { format!("{}    Cálculo: {}", optional_field(case.identity.full_name.as_deref())?, field(&case.name)?) } else { "Identificação oculta".into() }),
            format!("CPF: {}    Nascimento: {}", if options.include_personal { optional_field(case.identity.cpf.as_deref())? } else { "Oculto".into() }, date(case.identity.birth_date.as_deref())),
            format!("DIB: {}    Sexo: {}    Hipótese sujeita a revisão", date(Some(&case.dib)), enum_text(case.sex.as_deref(), &["masculino", "feminino", "Masculino", "Feminino"]))
        ],highlight: if matches!(options.kind, ReportKind::Detailed | ReportKind::Summary | ReportKind::Memory) {
            native.and_then(|n| n.pointer("/resumo/rmi")).filter(|v| !v.is_null()).map(|v| numeric(Some(v),true).map(|amount| format!("R$ {amount}"))).transpose()?
        } else { None },sections:vec![]};
    if let Some(n)=native.filter(|n|n.get("engine_id").and_then(Value::as_str)==Some("lc142-constant-grade-v1")){return pcd_report(case,workspace,options,n);}
    if report.office.chars().count()>120 || report.footer.chars().count()>180{return Err("Cabeçalho/rodapé excede o espaço reservado; nenhum texto foi cortado.".into());}
    if options.kind == ReportKind::Quality {
        report.sections.push(conclusion(case,options,native,outcome)?);
        report.sections.push(quality_calculation(case,options.include_personal)?);
        report.sections.push(pending(case,options.include_personal,outcome)?);
        return finish(report);
    }
    if options.kind == ReportKind::Analysis {
        report.sections.push(analysis_status(case,options,native)?);
        report.sections.push(analysis_financial(case,options,native)?);
        report.sections.push(analysis_roi(native)?);
        report.sections.push(pending(case,options.include_personal,outcome)?);
        return finish(report);
    }
    report.sections.push(summary(case,options,native)?);
    if matches!(options.kind,ReportKind::Detailed|ReportKind::Summary|ReportKind::Memory|ReportKind::Indicators){
        report.sections.push(conclusion(case,options,native,outcome)?);
    }
    if options.kind == ReportKind::Detailed { report.sections.push(interview(case,options.include_personal)?); }
    if selected(options,&[ReportKind::Periods]){report.sections.push(periods(case,options.include_personal,native)?);}
    if selected(options,&[ReportKind::Salaries]) || (options.kind == ReportKind::Memory && native.and_then(|n| n.get("correcao")).is_none_or(Value::is_null)) { report.sections.extend(salaries(case,options.include_personal)?.into_iter().filter(|s| options.kind != ReportKind::Memory || s.tables.iter().any(|t| !t.rows.is_empty()))); }
    if case.contribution_review.is_some() && selected(options,&[ReportKind::Salaries,ReportKind::Memory]){report.sections.push(contribution_ledger(case,options.include_personal)?);}
    if selected(options,&[ReportKind::Memory,ReportKind::Summary]){let mut sections=engine(native,options.include_personal)?;if options.kind==ReportKind::Summary{sections.truncate(1);}report.sections.extend(sections);}
    if selected(options,&[ReportKind::Transitions]){report.sections.push(transitions(native,options.include_personal)?);}
    if options.kind==ReportKind::Detailed {
        if let Some(fabrica)=case.fabrica.as_ref().filter(|value|!value.is_null()) {
            report.sections.push(fabrica_section(fabrica,case)?);
        }
    }
    // Summary has a compact warning cover; all other kinds include relevant pending records.
    if options.kind==ReportKind::Summary {let mut s=section("Ressalvas e conclusão",false);s.paragraphs.push("A fonte não foi automaticamente validada. Conferir PRPPS, datas finais, concomitância, indicadores e moedas históricas antes de usar um resultado para benefício.".into());if let Some(o)=outcome{let count=["blockers","missing","warnings","known_model_risks"].iter().map(|k|o.get(k).and_then(Value::as_array).map_or(0,Vec::len)).sum::<usize>();s.paragraphs.push(format!("Resultado fornecido registra {count} pendência(s)/aviso(s)/risco(s). O relatório detalhado mantém as categorias e origens."));}report.sections.push(s);}
    else{report.sections.push(pending(case,options.include_personal,outcome)?);}
    if matches!(options.kind,ReportKind::Detailed|ReportKind::Indicators){
        if options.presentation.include_quality && case.cnis.is_some(){report.sections.push(quality(case,options.include_personal)?);}
        if options.presentation.include_planning && is_planning(case){report.sections.push(planning(case,options.include_personal)?);}
    }
    // Domain reports lead with their own content. Import inventory belongs to the appendix.
    let mut inventory = report.sections.remove(0);
    let keep_appendix = options.kind == ReportKind::Detailed && options.presentation.include_appendix;
    if keep_appendix {
        inventory.title = "Anexo técnico - dados, origem e alcance".into();
        inventory.new_page = true;
        report.sections.push(inventory);
    } else if options.kind == ReportKind::Detailed {
        // Without the technical annex the synthesis stays in the body of the report.
        inventory.title = "Síntese, situação e alcance".into();
        inventory.new_page = false;
        report.sections.push(inventory);
    }
    report.sections.sort_by_key(|s| {
        if options.kind == ReportKind::Indicators {
            return match s.title.as_str() {
                "Conclusão e o que pode mudá-la" => 0,
                "Qualidade de segurado - conferência por vínculo" => 1,
                "Planejamento - premissas declaradas e lacunas" => 2,
                "Indicadores, pendências e ressalvas" => 3,
                "Anexo técnico - dados, origem e alcance" => 9,
                _ => 4,
            };
        }
        if options.kind == ReportKind::Transitions {
            return match s.title.as_str() {
                "Conclusão e o que pode mudá-la" => 0,
                "Análise das regras de transição" => 1,
                _ => 2,
            };
        }
        if options.kind == ReportKind::Memory {
            return match s.title.as_str() {
                "Conclusão e o que pode mudá-la" => 0,
                "Salários de contribuição - memória de atualização" if s.tables.iter().any(|t| !t.rows.is_empty()) => 1,
                "Cálculo da renda mensal inicial" => 2,
                "Memória de cálculo" => 3,
                _ if s.title.to_lowercase().contains("remunerações") || s.title.to_lowercase().contains("salários de contribuição") => 1,
                _ => 4,
            };
        }
        match s.title.as_str() {
            "Conclusão e o que pode mudá-la" => 0,
            "Cálculo da renda mensal inicial" => 1,
            "Períodos de contribuição" => 2,
            "Salários de contribuição - memória de atualização" => 3,
            "Análise das regras de transição" => 4,
            "Qualidade de segurado - conferência por vínculo" => 5,
            "Planejamento - premissas declaradas e lacunas" => 6,
            "Memória de cálculo" => 7,
            "Síntese, situação e alcance" => 8,
            "Entrevista do assistente e declarações" => 9,
            "Anexo técnico - dados, origem e alcance" => 10,
            _ => 10,
        }
    });
    for s in &mut report.sections {
        s.tables.retain(|t| !t.rows.is_empty());
        // A missing series is a short notice, never a nearly empty dedicated page.
        if s.title != "Anexo técnico - dados, origem e alcance" { s.new_page = false; }
    }
    report.sections.retain(|s| !s.paragraphs.is_empty() || !s.tables.is_empty());
    if let Some(first) = report.sections.first_mut() { first.new_page = false; }
    for line in &report.identification { field(line)?; }
    let mut total_rows=0usize;
    for s in &report.sections{for t in &s.tables{if t.headers.len()!=t.weights.len()||t.weights.contains(&0){return Err("Definição de tabela PDF inválida.".into());}for r in &t.rows{if r.len()!=t.headers.len(){return Err("Linha de tabela PDF inválida.".into());}total_rows+=1;for v in r{field(v)?;}}}for p in &s.paragraphs{field(p)?;}}
    if total_rows>MAX_ROWS{return Err("Documento excede 10.000 linhas de tabela; nenhum registro foi omitido.".into());}
    Ok(report)
}
/// Shared final checks for every report kind.
fn finish(report:Report)->Result<Report,String>{
    let mut report=report;
    for s in &mut report.sections {
        s.tables.retain(|t| !t.rows.is_empty());
        if s.title != "Anexo técnico - dados, origem e alcance" { s.new_page = false; }
    }
    report.sections.retain(|s| !s.paragraphs.is_empty() || !s.tables.is_empty());
    if let Some(first) = report.sections.first_mut() { first.new_page = false; }
    for line in &report.identification { field(line)?; }
    let mut total_rows=0usize;
    for s in &report.sections{for t in &s.tables{if t.headers.len()!=t.weights.len()||t.weights.contains(&0){return Err("Definição de tabela PDF inválida.".into());}for r in &t.rows{if r.len()!=t.headers.len(){return Err("Linha de tabela PDF inválida.".into());}total_rows+=1;for v in r{field(v)?;}}}for p in &s.paragraphs{field(p)?;}}
    if total_rows>MAX_ROWS{return Err("Documento excede 10.000 linhas de tabela; nenhum registro foi omitido.".into());}
    // Signature block: only when personal identification is included.
    if report.personal {
        let mut sig = section("Assinatura do responsavel", false);
        sig.paragraphs.push("".into());
        sig.paragraphs.push("_______________________________________________".into());
        sig.paragraphs.push("Nome do responsavel tecnico".into());
        sig.paragraphs.push("".into());
        sig.paragraphs.push("Data: ______/______/__________".into());
        report.sections.push(sig);
    }
    Ok(report)
}
#[cfg(test)]mod tests{use super::*;
    #[test]fn disability_aliases_do_not_become_false_pending_values(){
        for value in ["nao","não","none","sem_deficiencia","Não possui deficiência","nao possui deficiencia","Não","nenhuma"] {assert_eq!(declared_disability(Some(value)),"Sem deficiência declarada");}
        assert_eq!(declared_disability(None),"Não informado");
        assert_eq!(declared_disability(Some("Leve")),"Leve");
        assert_eq!(declared_disability(Some("PRIVATE TEXT")),"Valor fora do contrato / pendente");
    }
    #[test]fn private_pending_groups_only_hidden_text_and_preserves_counts(){
        let outcome=serde_json::json!({"warnings":["PRIVATE_A","PRIVATE_B",{"code":"source_end_missing","message":"PRIVATE_C"}],"data":{"native_result":{"avisos":["PRIVATE_D","PRIVATE_E"]}}});
        let private=pending(&Calculation::default(),false,Some(&outcome)).unwrap();
        let rows=&private.tables[0].rows;
        assert_eq!(rows.len(),2);
        assert!(rows.iter().any(|r|r[0]=="Aviso: 2 ocorrência(s)"));
        assert!(rows.iter().any(|r|r[1].contains("Data ausente/inválida")));
        assert_eq!(private.tables[1].rows[0][0],"Avisos do motor: 2");
        assert!(!format!("{private:?}").contains("PRIVATE_"));
        let personal=pending(&Calculation::default(),true,Some(&outcome)).unwrap();
        assert_eq!(personal.tables[0].rows.len(),3);
        assert_eq!(personal.tables[1].rows.len(),2);
        assert_eq!(personal.tables[0].rows[0][1],"PRIVATE_A");
    }
    #[test]fn exact_currency_not_float(){assert_eq!(cents(326940),"3.269,40");assert_eq!(cents(-1),"-0,01");assert_eq!(decimal("2048.18",true).unwrap(),"2.048,18");assert!(decimal("1.123",true).is_err());}
    #[test]fn null_bool_is_distinct(){assert_ne!(opt_bool(None),opt_bool(Some(false)));assert_eq!(opt_bool(None),"Não respondido");}
    #[test]fn date_is_checked(){assert!(iso_valid("2024-02-29"));assert!(!iso_valid("2023-02-29"));assert!(!iso_valid("1900-02-29"));assert!(!iso_valid("敏-01-01"));}
    #[test]fn text_limits_fail_not_clip(){assert!(field(&"a".repeat(1001)).is_err());assert!(field("a\0b").is_err());}
    #[test]fn numeric_not_arbitrary_text(){assert!(numeric(Some(&Value::String("CPF123".into())),true).is_err());}
    #[test]fn private_indicator_whitelist(){assert_eq!(indicators(&["PRPPS".into(),"SECRET NAME".into()],false).unwrap(),"PRPPS; 1 código(s) não catalogado(s) oculto(s)");}
}
