use crate::model::{Report, Table};
use genpdf::{elements, error::{Error, ErrorKind}, fonts, render, style::{Color, Style},
    Alignment, Context, Document, Element, Margins, Mm, PageDecorator, Position, RenderResult};
use std::io::{self, Write};
const REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");
const MAX_BYTES: usize = 64 * 1024 * 1024;
// Measured A4 references: x=42.52..569.02 pt; title baseline y=726.66 pt.
const CONTENT_WIDTH_MM: f64 = 185.7375;
const TITLE_BASELINE_PT: f64 = 785.0;
const BODY_TOP_MM: f64 = 41.0;
const BODY_BOTTOM_MM: f64 = 278.0;
const FOOTER_TOP_MM: f64 = 282.0;
const RULE_WIDTH_PT: f32 = 0.283;
const TABLE_BLUE: (u8, u8, u8) = (51, 67, 93);
const ROW_GRAY: (u8, u8, u8) = (246, 247, 249);
fn navy() -> Color { Color::Rgb(23, 44, 65) }
fn gray() -> Color { Color::Rgb(40, 48, 58) }
fn body_style() -> Style { Style::new().with_font_size(9).with_color(gray()) }
fn text(s: &str) -> elements::Paragraph { elements::Paragraph::new(wrap_tokens(s, 22)) }
/// genpdf does not automatically split an unbroken token. Insert whitespace only
/// between bounded chunks rather than let identifiers/tokens disappear off-page.
fn wrap_tokens(value: &str, limit: usize) -> String {
    value.split_whitespace().map(|word| {
        let chars: Vec<char> = word.chars().collect();
        chars.chunks(limit).map(|c| c.iter().collect::<String>()).collect::<Vec<_>>().join(" ")
    }).collect::<Vec<_>>().join(" ")
}
fn font_family(regular: &[u8], bold: &[u8]) -> Result<fonts::FontFamily<fonts::FontData>, String> {
    for data in [regular, bold] { if data.len() < 12 || !matches!(&data[..4], b"\x00\x01\x00\x00" | b"OTTO" | b"ttcf") { return Err("Cabeçalho de fonte incorporada inválido.".into()); } }
    let regular = fonts::FontData::new(regular.to_vec(), None)
        .map_err(|_| "Fonte regular incorporada inválida.".to_string())?;
    let bold = fonts::FontData::new(bold.to_vec(), None)
        .map_err(|_| "Fonte negrito incorporada inválida.".to_string())?;
    // No faux italic is used in this report; the other two slots are explicit aliases.
    Ok(fonts::FontFamily { regular: regular.clone(), bold: bold.clone(), italic: regular, bold_italic: bold })
}
// Prefer the same installed typeface as the reference PDFs; never redistribute OS fonts.
pub(crate) fn report_fonts() -> (Vec<u8>, Vec<u8>) {
    #[cfg(target_os = "windows")]
    {
        let directory = std::env::var_os("WINDIR").map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("C:\\Windows")).join("Fonts");
        let regular = std::fs::read(directory.join("trebuc.ttf"));
        let bold = std::fs::read(directory.join("trebucbd.ttf"));
        if let (Ok(regular), Ok(bold)) = (regular, bold) {
            if regular.len() <= 5 * 1024 * 1024 && bold.len() <= 5 * 1024 * 1024
                && font_family(&regular, &bold).is_ok() { return (regular, bold); }
        }
    }
    (REGULAR.to_vec(), BOLD.to_vec())
}
fn preflight(report: &Report, regular: &[u8], bold_bytes: &[u8]) -> Result<(), String> {
    let font = rusttype::Font::from_bytes(regular.to_vec()).map_err(|_| "Fonte inválida.".to_string())?;
    let bold = rusttype::Font::from_bytes(bold_bytes.to_vec()).map_err(|_| "Fonte inválida.".to_string())?;
    let mut all = vec![report.title.as_str(), report.office.as_str(), report.footer.as_str(), report.status.as_str()];
    all.extend(report.identification.iter().map(String::as_str));
    if let Some(value) = &report.highlight { all.push(value); }
    for section in &report.sections {
        all.push(&section.title);
        for p in &section.paragraphs { all.push(p); }
        for table in &section.tables {
            all.push(&table.caption);
            for h in &table.headers { all.push(h); }
            for row in &table.rows { for cell in row { all.push(cell); } }
        }
    }
    for value in all {
        for c in value.chars().filter(|c| !c.is_whitespace()) {
            if font.glyph(c).id().0 == 0 || bold.glyph(c).id().0 == 0 {
                return Err("Caractere não suportado pelas fontes PDF incorporadas; revisar o texto. Nenhum caractere foi descartado.".into());
            }
        }
    }
    Ok(())
}
struct Decorator { page: usize, total_pages: Option<usize>, title: String, office: String, footer: String, private: bool, identification: [String; 3] }
impl PageDecorator for Decorator {
    fn decorate_page<'a>(&mut self, context: &Context, mut area: render::Area<'a>, style: Style)
        -> Result<render::Area<'a>, Error> {
        self.page += 1;
        if self.page > 1500 { return Err(Error::new("Limite de páginas PDF excedido", ErrorKind::InvalidData)); }
        area.add_margins(Margins::trbl(0, 210.0 - 15.0 - CONTENT_WIDTH_MM, 0, 15));
        let mut office_area = area.clone();
        office_area.add_offset(Position::new(0, 7));
        office_area.set_height(Mm::from(9));
        let office_result = text(&self.office).aligned(Alignment::Center)
            .styled(Style::new().bold().with_font_size(10).with_color(navy()))
            .render(context, office_area, style)?;
        if office_result.has_more { return Err(Error::new("Cabeçalho excede espaço reservado", ErrorKind::InvalidData)); }
        let title_style = Style::new().bold().with_font_size(12).with_color(navy());
        let title_top = Mm::from((841.89 - TITLE_BASELINE_PT) * 25.4 / 72.0)
            - title_style.font(&context.font_cache).glyph_height(12);
        let mut title_area = area.clone();
        title_area.add_offset(Position::new(0, title_top));
        title_area.set_height(Mm::from(7));
        let title_result = text(&self.title).aligned(Alignment::Center).styled(title_style)
            .render(context, title_area, style)?;
        if title_result.has_more { return Err(Error::new("Título excede espaço reservado", ErrorKind::InvalidData)); }
        let mut band = area.clone();
        band.add_offset(Position::new(0, 25));
        band.set_height(Mm::from(13));
        let size = band.size();
        band.draw_line(vec![Position::new(0,0), Position::new(size.width,0),
            Position::new(size.width,size.height), Position::new(0,size.height), Position::new(0,0)],
            Style::new().with_color(Color::Rgb(224,224,224)));
        let mut identifiers = elements::LinearLayout::vertical();
        for line in &self.identification {
            identifiers.push(text(line).styled(Style::new().with_font_size(8).with_color(gray())));
        }
        let identity_result = identifiers.padded(Margins::trbl(1.0,1.5,1.0,1.5)).render(context, band, style)?;
        if identity_result.has_more { return Err(Error::new("Identificação excede a faixa do relatório", ErrorKind::InvalidData)); }
        let mut footer_area = area.clone();
        footer_area.add_offset(Position::new(0, FOOTER_TOP_MM));
        footer_area.set_height(Mm::from(12));
        let mut footer = elements::LinearLayout::vertical();
        footer.push(text(&self.footer).styled(Style::new().with_font_size(7).with_color(gray())));
        let page_label = match self.total_pages {
            Some(total) => format!("Página {}/{total}", self.page),
            None => format!("Página {}", self.page),
        };
        footer.push(text(&format!("{page_label} | {} | Revisão necessária", if self.private {"IDENTIFICAÇÃO OCULTA"} else {"USO CONFIDENCIAL"}))
            .aligned(Alignment::Center).styled(Style::new().with_font_size(7).with_color(gray())));
        let footer_result = footer.render(context, footer_area, style)?;
        if footer_result.has_more { return Err(Error::new("Rodapé excede espaço reservado", ErrorKind::InvalidData)); }
        area.add_offset(Position::new(0, BODY_TOP_MM));
        area.set_height(Mm::from(BODY_BOTTOM_MM - BODY_TOP_MM));
        Ok(area)
    }
}
/// Repeats an actual table header at each continuation render. genpdf 0.2.0
/// has no built-in table header callback, so this wrapper owns that contract.
struct RepeatingTable { headers: Vec<String>, weights: Vec<usize>, table: elements::TableLayout, done: bool }
struct WidthWrappedCell { value: String, header: bool, produced: bool }
/// Wraps a cell value so that EVERY word genpdf measures (the word plus the
/// separating space it keeps) fits the column. genpdf silently drops a word
/// that is wider than the line, which would erase a date or a code from the
/// document without any error.
fn wrap_cell_text(value: &str, width: Mm, measure: impl Fn(&str) -> Mm) -> Result<String, String> {
    if width <= Mm::from(0) { return Err("Coluna sem largura útil".into()); }
    let mut wrapped = String::new();
    for word in value.split_whitespace() {
        let mut chunk = String::new();
        for c in word.chars() {
            let mut next = chunk.clone(); next.push(c);
            let mut probe = next.clone(); probe.push(' ');
            if measure(&probe) > width {
                if chunk.is_empty() { return Err("Caractere excede largura da coluna".into()); }
                wrapped.push_str(&chunk); wrapped.push(' '); chunk.clear();
            }
            chunk.push(c);
        }
        wrapped.push_str(&chunk); wrapped.push(' ');
    }
    Ok(wrapped.trim_end().to_string())
}
impl Element for WidthWrappedCell {
    fn render(&mut self, context: &Context, area: render::Area<'_>, style: Style) -> Result<RenderResult, Error> {
        // The paragraph is rebuilt on every render: genpdf consumes the words of
        // a paragraph, so a cached one would come back empty when a row that
        // was cut by the page break is drawn again.
        // The separating space is part of the word genpdf measures, so it has to
        // be part of our measurement too.
        let wrapped = wrap_cell_text(&self.value, area.size().width, |text| style.str_width(&context.font_cache, text))
            .map_err(|message| Error::new(message, ErrorKind::InvalidData))?;
        let numeric = self.value.chars().any(|c| c.is_ascii_digit())
            && self.value.chars().all(|c| c.is_ascii_digit() || ".,%+- /".contains(c));
        let alignment = if self.header || (numeric && self.value.contains('/')) { Alignment::Center }
            else if numeric { Alignment::Right } else { Alignment::Left };
        let mut paragraph = elements::Paragraph::new(wrapped).aligned(alignment);
        let rendered = paragraph.render(context, area, style)?;
        if rendered.size.height > Mm::from(0) { self.produced = true; }
        else if !self.produced && !rendered.has_more && !self.value.trim().is_empty() {
            // genpdf silently drops a word that is wider than the line. With the
            // wrap above that cannot happen, and if it ever does the document is
            // refused instead of publishing an empty cell.
            return Err(Error::new(format!("Célula sem espaço: [{}]",self.value), ErrorKind::InvalidData));
        }
        Ok(rendered)
    }
}
struct ReferenceCells { header: bool, frame: elements::FrameCellDecorator }
impl ReferenceCells {
    fn new(header: bool) -> Self { Self { header, frame: elements::FrameCellDecorator::new(true, true, true) } }
}
impl elements::CellDecorator for ReferenceCells {
    fn set_table_size(&mut self, columns: usize, rows: usize) { self.frame.set_table_size(columns, rows); }
    fn decorate_cell(&mut self, column: usize, row: usize, has_more: bool, area: render::Area<'_>, style: Style) {
        if self.header || row % 2 == 1 {
            let (r,g,b) = if self.header { TABLE_BLUE } else { ROW_GRAY };
            let size = area.size();
            // genpdf exposes strokes only. Finalization fills these closed bands
            // behind the text, preserving every glyph and continuation fragment.
            area.draw_line(vec![Position::new(0,0), Position::new(size.width,0),
                Position::new(size.width,size.height), Position::new(0,size.height), Position::new(0,0)],
                Style::new().with_color(Color::Rgb(r,g,b)));
        }
        let color = if self.header { Color::Rgb(255,255,255) } else { Color::Rgb(192,192,192) };
        self.frame.decorate_cell(column, row, has_more, area, style.with_color(color));
    }
}
fn row(table: &mut elements::TableLayout, values: &[String], header: bool) -> Result<(), String> {
    let style = Style::new().with_font_size(9).with_line_spacing(1.0)
        .with_color(if header { Color::Rgb(255,255,255) } else { gray() });
    let style = if header { style.bold() } else { style };
    let mut row = table.row();
    for value in values {
        let vertical = if header { 1.25 } else { 0.55 };
        row.push_element(WidthWrappedCell { value: value.clone(), header, produced: false }.styled(style)
            .padded(Margins::trbl(vertical, 1, vertical, 1)));
    }
    row.push().map_err(|_| "Estrutura de tabela PDF inválida.".to_string())
}
impl RepeatingTable {
    fn new(data: &Table) -> Result<Self, String> {
        let mut table = elements::TableLayout::new(data.weights.clone());
        table.set_cell_decorator(ReferenceCells::new(false));
        for values in &data.rows { row(&mut table, values, false)?; }
        Ok(Self { headers: data.headers.clone(), weights: data.weights.clone(), table, done: false })
    }
}
impl Element for RepeatingTable {
    fn render(&mut self, context: &Context, mut area: render::Area<'_>, style: Style) -> Result<RenderResult, Error> {
        if self.done { return Ok(RenderResult::default()); }
        if area.size().height < Mm::from(20) { return Ok(RenderResult { has_more: true, ..Default::default() }); }
        let mut head = elements::TableLayout::new(self.weights.clone());
        head.set_cell_decorator(ReferenceCells::new(true));
        row(&mut head, &self.headers, true).map_err(|_| Error::new("Cabeçalho de tabela inválido", ErrorKind::InvalidData))?;
        let hr = head.render(context, area.clone(), style)?;
        if hr.has_more { return Err(Error::new("Cabeçalho de tabela excede página", ErrorKind::InvalidData)); }
        area.add_offset(Position::new(0, hr.size.height));
        let mut result = self.table.render(context, area, style)?;
        result.size.height += hr.size.height;
        self.done = !result.has_more;
        Ok(result)
    }
}
/// Keep a caption with its column headings and at least one ordinary row.
/// The table itself retains its streaming continuation and repeated headings.
struct CaptionedTable { content: elements::LinearLayout, started: bool }
impl CaptionedTable {
    fn new(table: &Table) -> Result<Self, String> {
        let mut content = elements::LinearLayout::vertical();
        if !table.caption.is_empty() {
            content.push(text(&table.caption).styled(Style::new().bold().with_font_size(9).with_color(navy())));
            content.push(elements::Break::new(0.15));
        }
        if table.rows.is_empty() {
            content.push(text("Nenhum registro disponível nesta seção.").styled(body_style()));
        } else {
            content.push(RepeatingTable::new(table)?);
        }
        Ok(Self { content, started: false })
    }
}
impl Element for CaptionedTable {
    fn render(&mut self, context: &Context, area: render::Area<'_>, style: Style) -> Result<RenderResult, Error> {
        if !self.started && area.size().height < Mm::from(27) {
            return Ok(RenderResult { has_more: true, ..Default::default() });
        }
        self.started = true;
        self.content.render(context, area, style)
    }
}
#[derive(Default)]
struct BoundedWriter(Vec<u8>);
impl Write for BoundedWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.0.len().checked_add(data.len()).is_none_or(|v| v > MAX_BYTES) {
            return Err(io::Error::other("PDF excede limite de 64 MiB"));
        }
        self.0.extend_from_slice(data); Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
pub(crate) fn render(report: Report) -> Result<Vec<u8>, String> {
    let (regular, bold) = report_fonts();
    preflight(&report, &regular, &bold)?;
    // First pass: render without total to count pages.
    let first = build_and_render(&report, &regular, &bold, None)?;
    let page_count = lopdf::Document::load_mem(&first)
        .map(|doc| doc.get_pages().len())
        .unwrap_or(0);
    if page_count == 0 {
        return Err("PDF sem paginas apos a primeira passagem.".into());
    }
    // Second pass: render with total page count.
    build_and_render(&report, &regular, &bold, Some(page_count))
}

fn build_and_render(report: &Report, regular: &[u8], bold: &[u8], total_pages: Option<usize>)
    -> Result<Vec<u8>, String> {
    let fonts = font_family(regular, bold)?;
    let mut doc = Document::new(fonts);
    doc.set_title(report.title.clone());
    doc.set_paper_size((210, 297)); doc.set_font_size(9); doc.set_line_spacing(1.0);
    doc.set_minimal_conformance();
    doc.set_page_decorator(Decorator { page: 0, total_pages, title: report.title.clone(), office: report.office.clone(),
        footer: report.footer.clone(), private: !report.personal, identification: report.identification.clone() });
    doc.push(text(&report.status).styled(Style::new().with_font_size(8).with_color(gray())));
    if let Some(value) = &report.highlight {
        doc.push(elements::Break::new(0.5));
        doc.push(text("Renda mensal inicial estimada").styled(Style::new().with_font_size(10).with_color(gray())));
        doc.push(text(value).styled(Style::new().bold().with_font_size(22).with_color(navy())));
        doc.push(text("Valor do cenario declarado. Confira a memoria e as pendencias abaixo.").styled(body_style()));
        doc.push(elements::Break::new(0.7));
    }
    for section in &report.sections {
        if section.new_page { doc.push(elements::PageBreak::new()); }
        doc.push(elements::Break::new(0.5));
        doc.push(text(&section.title)
            .styled(Style::new().bold().with_font_size(12).with_color(navy())));
        doc.push(elements::Break::new(0.3));
        for paragraph in &section.paragraphs { doc.push(text(paragraph).styled(body_style())); doc.push(elements::Break::new(0.35)); }
        for table in &section.tables {
            doc.push(CaptionedTable::new(table)?);
            doc.push(elements::Break::new(0.6));
        }
    }
    let mut writer = BoundedWriter::default();
    doc.render(&mut writer).map_err(|e| format!("Falha ao diagramar o PDF (limites de pagina, fonte ou tamanho); nenhum relatorio valido foi produzido. [{}]",e))?;
    if !writer.0.starts_with(b"%PDF-") || writer.0.len() < 500 { return Err("Saida PDF invalida ou vazia.".into()); }
    normalize_unicode_maps(&writer.0)
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn invalid_font_is_error() { assert!(font_family(b"not a font", BOLD).is_err()); assert!(font_family(REGULAR, &[]).is_err()); }
    #[test] fn writer_is_bounded() { let mut w=BoundedWriter(vec![0;MAX_BYTES]); assert!(w.write(&[1]).is_err()); }
    #[test] fn tokens_have_break_opportunities() { assert!(wrap_tokens(&"a".repeat(90),14).split_whitespace().all(|w|w.len()<=14)); }
    #[test] fn narrow_cell_words_never_exceed_the_column_width() {
        let width=Mm::from(19);
        // Two units per character: a ten character date does not fit and has to
        // be split, an eight character one does.
        let measure=|text:&str|Mm::from(text.chars().count() as f64*2.0);
        for value in ["15/10/2021","2021-01-15","15102021","abcdefghijklmnop","Mais de 120 contribuições"] {
            let wrapped=wrap_cell_text(value,width,measure).unwrap();
            assert!(!wrapped.is_empty(),"nothing wrapped for {value}");
            for word in wrapped.split(' ') {
                let piece=format!("{word} ");
                assert!(measure(&piece)<=width,"word wider than the column: {piece}");
            }
        }
        assert!(wrap_cell_text("15/10/2021",width,measure).unwrap().contains(' '),"a date that does not fit must be split");
        assert!(wrap_cell_text("15102021",width,measure).unwrap().contains(' ').eq(&false),"a short code must not be split");
        assert!(wrap_cell_text("x",Mm::from(0),measure).is_err());
    }
}

/// printpdf 0.3.4 writes non-BMP cmap scalars as five/six hex digits instead of
/// UTF-16BE and can emit empty bfchar blocks. Normalize OUR generated ToUnicode
/// maps, without touching the input CNIS or any external PDF/font files.
/// Trebuchet aliases Unicode hyphen/nbsp glyphs; keep generated text canonically searchable.
fn normalize_unicode_maps(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc=lopdf::Document::load_mem(bytes).map_err(|_| "PDF intermediário inválido.".to_string())?;
    let mut ids=std::collections::BTreeSet::new();
    for object in doc.objects.values() {
        if let Ok(dict)=object.as_dict(){if let Ok(value)=dict.get(b"ToUnicode"){if let Ok(id)=value.as_reference(){ids.insert(id);}}}
    }
    if ids.is_empty(){return Err("PDF sem mapas Unicode incorporados.".into());}
    for id in ids {
        let stream=doc.get_object_mut(id).and_then(lopdf::Object::as_stream_mut).map_err(|_|"Mapa Unicode de fonte inválido.".to_string())?;
        let data=stream.decompressed_content().unwrap_or_else(|_|stream.content.clone());
        let source=std::str::from_utf8(&data).map_err(|_|"CMap intermediário ilegível.".to_string())?;
        let mut inside=false;let mut entries=vec![];
        for line in source.lines(){let line=line.trim();if line.ends_with("beginbfchar"){inside=true;continue;}if line=="endbfchar"{inside=false;continue;}if !inside{continue;}
            let tokens:Vec<_>=line.split_whitespace().collect();if tokens.len()!=2{return Err("CMap intermediário fora do contrato.".into());}
            let hex=|s:&str|->Result<u32,String>{let s=s.strip_prefix('<').and_then(|s|s.strip_suffix('>')).ok_or("CMap sem hexadecimal.")?;u32::from_str_radix(s,16).map_err(|_|"Valor CMap inválido.".into())};
            let glyph=hex(tokens[0])?;let scalar=match hex(tokens[1])? { 0x00AD | 0x2010 | 0x2011 => 0x002D, 0x00A0 => 0x0020, value => value };if glyph>65535{return Err("Glyph CMap excede contrato.".into());}
            let c=char::from_u32(scalar).ok_or("Scalar Unicode inválido no CMap.")?;
            let mut buf=[0u16;2];let unicode=c.encode_utf16(&mut buf).iter().map(|v|format!("{v:04X}")).collect::<String>();
            entries.push(format!("<{glyph:04X}> <{unicode}>\n"));
        }
        if entries.is_empty(){return Err("CMap sem entradas de fonte.".into());}
        let mut content=String::from("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /PreviusUTF16 def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");
        for chunk in entries.chunks(100){content.push_str(&format!("{} beginbfchar\n",chunk.len()));for mapping in chunk{content.push_str(mapping);}content.push_str("endbfchar\n");}
        content.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
        stream.dict.remove(b"Filter");stream.dict.remove(b"DecodeParms");stream.set_content(content.into_bytes());
    }
    if let Ok(id)=doc.trailer.get(b"Info").and_then(lopdf::Object::as_reference){if let Ok(info)=doc.get_object_mut(id).and_then(lopdf::Object::as_dict_mut){info.remove(b"Author");info.set("Creator",lopdf::Object::string_literal("Previus Rust native report renderer"));info.set("Producer",lopdf::Object::string_literal("genpdf 0.2.0; UTF-16 CMap normalization"));}}
    reference_paint(&mut doc)?;
    let mut output=BoundedWriter::default();doc.save_to(&mut output).map_err(|_|"Falha ao finalizar PDF Unicode ou limite de 64 MiB excedido.".to_string())?;
    Ok(output.0)
}

fn reference_paint(doc: &mut lopdf::Document) -> Result<(), String> {
    use lopdf::{content::{Content, Operation}, Object};
    for id in doc.get_pages().values() {
        let data = doc.get_page_content(*id).map_err(|_| "Conteúdo PDF inválido.".to_string())?;
        let content = Content::decode(&data).map_err(|_| "Operações PDF inválidas.".to_string())?;
        let mut foreground = vec![Operation::new("w", vec![Object::Real(RULE_WIDTH_PT)])];
        let mut background = Vec::new();
        let mut stroke = Vec::new();
        let mut path_start = 0;
        for mut op in content.operations {
            if op.operator == "RG" { stroke = op.operands.iter().filter_map(|v| v.as_float().ok()).collect(); }
            if op.operator == "m" { path_start = foreground.len(); }
            if op.operator == "w" { op.operands = vec![Object::Real(RULE_WIDTH_PT)]; }
            if (op.operator == "RG" || op.operator == "rg") && op.operands.len() == 3 {
                // printpdf rounds RGB commands to two decimals; recover our
                // known reference palette before lopdf writes the final stream.
                for &(r,g,b) in &[TABLE_BLUE, ROW_GRAY, (224,224,224), (40,48,58), (23,44,65), (192,192,192)] {
                    let rgb = [r,g,b];
                    if op.operands.iter().zip(rgb).all(|(a,b)| a.as_float().is_ok_and(|v|
                        (v - (f32::from(b)/255.0*100.0).round()/100.0).abs() < 0.0001)) {
                        op.operands = rgb.into_iter().map(|v| Object::Real(f32::from(v)/255.0)).collect();
                        break;
                    }
                }
            }
            let band = [TABLE_BLUE, ROW_GRAY, (224,224,224)].iter().any(|&(r,g,b)| {
                stroke.len() == 3 && stroke.iter().zip([r,g,b]).all(|(a,b)| (*a - (f32::from(b) / 255.0 * 100.0).round() / 100.0).abs() < 0.0001)
            });
            if op.operator == "S" && band && foreground.len() == path_start + 5 {
                let path = &foreground[path_start..];
                let points: Option<Vec<(f32,f32)>> = path.iter().enumerate().map(|(i,p)| {
                    if p.operator != if i == 0 { "m" } else { "l" } || p.operands.len() != 2 { return None; }
                    Some((p.operands[0].as_float().ok()?, p.operands[1].as_float().ok()?))
                }).collect();
                if let Some(p) = points.filter(|p| p[0] == p[4] && p[0].1 == p[1].1 && p[1].0 == p[2].0 && p[2].1 == p[3].1 && p[3].0 == p[0].0) {
                    background.push(Operation::new("q", vec![]));
                    let rgb = [TABLE_BLUE, ROW_GRAY, (224,224,224)].into_iter().find(|&(r,g,b)|
                        stroke.iter().zip([r,g,b]).all(|(a,b)| (*a - (f32::from(b)/255.0*100.0).round()/100.0).abs() < 0.0001))
                        .ok_or("Cor de faixa PDF não reconhecida.")?;
                    background.push(Operation::new("rg", [rgb.0,rgb.1,rgb.2].into_iter()
                        .map(|v| Object::Real(f32::from(v)/255.0)).collect()));
                    background.push(Operation::new("re", vec![p[0].0.into(),p[2].1.into(),(p[1].0-p[0].0).into(),(p[0].1-p[2].1).into()]));
                    background.push(Operation::new("f", vec![]));
                    background.push(Operation::new("Q", vec![]));
                    foreground.truncate(path_start);
                    continue;
                }
            }
            foreground.push(op);
        }
        background.extend(foreground);
        let data = Content { operations: background }.encode().map_err(|_| "Pintura PDF inválida.".to_string())?;
        doc.change_page_content(*id, data).map_err(|_| "Falha ao finalizar estilo PDF.".to_string())?;
    }
    Ok(())
}
