use crossterm::event::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyIntent {
    MoveDown,
    MoveUp,
    FocusNext,
    FocusPrevious,
    DrillIn,
    Back,
    Refresh,
    Checkout,
    Help,
    PageDown,
    PageUp,
    OpenExternal,
    DismissMessage,
}

pub(crate) fn key_intent(code: KeyCode) -> Option<KeyIntent> {
    match code {
        KeyCode::Down | KeyCode::Char('j') => Some(KeyIntent::MoveDown),
        KeyCode::Up | KeyCode::Char('k') => Some(KeyIntent::MoveUp),
        KeyCode::Tab | KeyCode::Char('l') => Some(KeyIntent::FocusNext),
        KeyCode::BackTab | KeyCode::Char('h') => Some(KeyIntent::FocusPrevious),
        KeyCode::Enter => Some(KeyIntent::DrillIn),
        KeyCode::Esc | KeyCode::Char('q') => Some(KeyIntent::Back),
        KeyCode::Char('r') => Some(KeyIntent::Refresh),
        KeyCode::Char('c') => Some(KeyIntent::Checkout),
        KeyCode::Char('?') => Some(KeyIntent::Help),
        KeyCode::PageDown | KeyCode::Char(' ') => Some(KeyIntent::PageDown),
        KeyCode::PageUp | KeyCode::Backspace => Some(KeyIntent::PageUp),
        KeyCode::Char('O') => Some(KeyIntent::OpenExternal),
        KeyCode::Char('x') => Some(KeyIntent::DismissMessage),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_global_navigation_keys() {
        assert_eq!(key_intent(KeyCode::Char('j')), Some(KeyIntent::MoveDown));
        assert_eq!(key_intent(KeyCode::Down), Some(KeyIntent::MoveDown));
        assert_eq!(key_intent(KeyCode::Char('k')), Some(KeyIntent::MoveUp));
        assert_eq!(key_intent(KeyCode::Up), Some(KeyIntent::MoveUp));
        assert_eq!(key_intent(KeyCode::Tab), Some(KeyIntent::FocusNext));
        assert_eq!(key_intent(KeyCode::BackTab), Some(KeyIntent::FocusPrevious));
        assert_eq!(key_intent(KeyCode::Enter), Some(KeyIntent::DrillIn));
        assert_eq!(key_intent(KeyCode::Esc), Some(KeyIntent::Back));
        assert_eq!(key_intent(KeyCode::Char('q')), Some(KeyIntent::Back));
        assert_eq!(key_intent(KeyCode::Char('c')), Some(KeyIntent::Checkout));
        assert_eq!(key_intent(KeyCode::Char('?')), Some(KeyIntent::Help));
        assert_eq!(
            key_intent(KeyCode::Char('x')),
            Some(KeyIntent::DismissMessage)
        );
    }
}
