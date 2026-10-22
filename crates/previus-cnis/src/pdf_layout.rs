//! Geometric text reconstruction for CNIS PDFs whose content stream emits
//! labels and values separately. Do not persist this intermediate text.
use pdf_extract::{MediaBox, OutputDev, OutputError, Transform};
#[derive(Default)]
pub(crate) struct PositionedText { glyphs: Vec<Glyph>, height: f64, width: f64, rotation: i64 }
struct Glyph { x: f64, y: f64, end_x: f64, size: f64, text: String }
impl OutputDev for PositionedText {
    fn begin_page(&mut self, _: u32, b: &MediaBox, _: Option<(f64,f64,f64,f64)>) -> Result<(),OutputError> {
        self.height = b.ury; self.width = b.urx; Ok(())
    }
    fn end_page(&mut self) -> Result<(),OutputError> { Ok(()) }
    fn output_character(&mut self, t: &Transform, width: f64, _: f64, font_size: f64, text: &str) -> Result<(),OutputError> {
        let size = ((t.m11 * font_size).powi(2) + (t.m12 * font_size).powi(2)).sqrt();
        if !t.m31.is_finite() || !t.m32.is_finite() || !size.is_finite() || self.glyphs.len() >= 2_000_000 {
            return Err(OutputError::FormatError(std::fmt::Error));
        }
        let (x,y) = match self.rotation {
            90 => (t.m32, t.m31),
            180 => (self.width-t.m31, t.m32),
            270 => (self.height-t.m32, self.width-t.m31),
            _ => (t.m31,self.height-t.m32),
        };
        self.glyphs.push(Glyph { x, y, end_x:x+width*size, size, text:text.into() });
        Ok(())
    }
    fn begin_word(&mut self) -> Result<(),OutputError> { Ok(()) }
    fn end_word(&mut self) -> Result<(),OutputError> { Ok(()) }
    fn end_line(&mut self) -> Result<(),OutputError> { Ok(()) }
}
impl PositionedText {
    pub(crate) fn new(rotation: i64) -> Self { Self { rotation: rotation.rem_euclid(360), ..Self::default() } }
    pub(crate) fn into_text(mut self) -> String {
        self.glyphs.sort_by(|a,b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
        let mut rows: Vec<Vec<Glyph>> = Vec::new();
        for glyph in self.glyphs {
            if let Some(row) = rows.last_mut() {
                if (row[0].y - glyph.y).abs() <= 1.5 { row.push(glyph); continue; }
            }
            rows.push(vec![glyph]);
        }
        let mut output = String::new();
        for mut row in rows {
            row.sort_by(|a,b| a.x.total_cmp(&b.x));
            let mut previous: Option<(f64,f64)> = None;
            for glyph in row {
                if let Some((end,size)) = previous {
                    let gap = glyph.x - end;
                    if gap > size.min(glyph.size)*0.18 && !output.ends_with(' ') && !glyph.text.starts_with(' ') {
                        // Keep the horizontal distance between columns observable:
                        // one space per gap unit, so a later parser can tell
                        // which column a value belongs to.
                        let unit = size.min(glyph.size)*0.35;
                        let spaces = if unit > 0.0 { ((gap/unit).round() as usize).clamp(1,40) } else { 1 };
                        for _ in 0..spaces { output.push(' '); }
                    }
                }
                output.push_str(&glyph.text);
                previous = Some((glyph.end_x,glyph.size));
            }
            output.push('\n');
        }
        output
    }
}

/// /Rotate may be inherited through page-tree parents.
pub(crate) fn page_rotation(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> i64 {
    let mut id = page_id;
    for _ in 0..64 {
        let Ok(dict) = doc.get_dictionary(id) else { break; };
        if let Ok(rotation) = dict.get(b"Rotate").and_then(lopdf::Object::as_i64) { return rotation; }
        let Ok(parent) = dict.get(b"Parent").and_then(lopdf::Object::as_reference) else { break; };
        id = parent;
    }
    0
}
