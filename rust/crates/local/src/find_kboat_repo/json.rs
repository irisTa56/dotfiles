//! JSON text laid out again, keys kept in the order the text has them.

use std::io;

/// JSON text, laid out again with its keys in the order the text has them.
pub(super) struct Json<'a>(&'a str);

impl<'a> Json<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        Json(text)
    }

    /// The text with no whitespace between tokens.
    pub(super) fn compact(&self) -> serde_json::Result<String> {
        let mut out = Vec::new();
        self.transcode(&mut serde_json::Serializer::new(&mut out))?;
        Ok(String::from_utf8(out).expect("serde_json writes UTF-8"))
    }

    /// The text indented by two spaces and ended by a newline.
    pub(super) fn pretty(&self) -> serde_json::Result<String> {
        let mut out = Vec::new();
        self.transcode(&mut serde_json::Serializer::pretty(&mut out))?;
        out.push(b'\n');
        Ok(String::from_utf8(out).expect("serde_json writes UTF-8"))
    }

    fn transcode<W: io::Write, F: serde_json::ser::Formatter>(
        &self,
        serializer: &mut serde_json::Serializer<W, F>,
    ) -> serde_json::Result<()> {
        let mut deserializer = serde_json::Deserializer::from_str(self.0);
        serde_transcode::transcode(&mut deserializer, serializer)?;
        deserializer.end()
    }
}
