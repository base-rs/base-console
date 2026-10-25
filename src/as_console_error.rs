// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use std::error::Error;

/// Macro support for concrete errors and autodereferenced boxed or borrowed trait objects.
pub trait AsConsoleError<'a> {
    fn console_error(self) -> &'a (dyn Error + 'static);
}

impl<'a, E: Error + 'static> AsConsoleError<'a> for &'a E {
    fn console_error(self) -> &'a (dyn Error + 'static) {
        self
    }
}

impl<'a> AsConsoleError<'a> for &'a (dyn Error + 'static) {
    fn console_error(self) -> &'a (dyn Error + 'static) {
        self
    }
}

impl<'a> AsConsoleError<'a> for &'a (dyn Error + Send + 'static) {
    fn console_error(self) -> &'a (dyn Error + 'static) {
        self
    }
}

impl<'a> AsConsoleError<'a> for &'a (dyn Error + Sync + 'static) {
    fn console_error(self) -> &'a (dyn Error + 'static) {
        self
    }
}

impl<'a> AsConsoleError<'a> for &'a (dyn Error + Send + Sync + 'static) {
    fn console_error(self) -> &'a (dyn Error + 'static) {
        self
    }
}
