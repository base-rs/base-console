// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

/// The representation written by the console.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OutputFormat {
    /// Text for a terminal, JSON for a redirected stream or custom writer.
    #[default]
    Auto,
    /// Human-readable text, including indented causes and captured diagnostics.
    Text,
    /// One structured JSON object per line.
    Json,
}

impl OutputFormat {
    pub(crate) fn resolve(self, terminal: bool) -> Self {
        match self {
            Self::Auto if terminal => Self::Text,
            Self::Auto => Self::Json,
            format => format,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OutputFormat::*;

    #[test]
    fn automatic_selection_tracks_the_destination_and_explicit_formats_win() {
        assert_eq!(Auto.resolve(true), Text);
        assert_eq!(Auto.resolve(false), Json);
        for terminal in [true, false] {
            assert_eq!(Text.resolve(terminal), Text);
            assert_eq!(Json.resolve(terminal), Json);
        }
    }
}
