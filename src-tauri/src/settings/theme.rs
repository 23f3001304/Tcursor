use crate::settings::model::ThemeMode;

pub fn resolve_dark(theme: ThemeMode, os_dark: bool) -> bool {
    match theme {
        ThemeMode::Light => false,
        ThemeMode::Dark => true,
        ThemeMode::System => os_dark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_is_not_dark() {
        assert!(!resolve_dark(ThemeMode::Light, true));
    }

    #[test]
    fn dark_is_dark() {
        assert!(resolve_dark(ThemeMode::Dark, false));
    }

    #[test]
    fn system_follows_the_desktop() {
        assert!(resolve_dark(ThemeMode::System, true));
        assert!(!resolve_dark(ThemeMode::System, false));
    }
}
