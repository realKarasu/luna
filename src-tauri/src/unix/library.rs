//! Installed games are read from the Windows game stores. Elsewhere the list is empty.

use crate::library::InstalledGame;

pub fn installed() -> Vec<InstalledGame> {
    Vec::new()
}
