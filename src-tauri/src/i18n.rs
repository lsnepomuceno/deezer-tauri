#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Labels {
    pub nothing_playing: &'static str,
    pub play: &'static str,
    pub pause: &'static str,
    pub next: &'static str,
    pub previous: &'static str,
    pub show_window: &'static str,
    pub hide_window: &'static str,
    pub close_to_tray: &'static str,
    pub notifications: &'static str,
    pub quit: &'static str,
}

pub const EN: Labels = Labels {
    nothing_playing: "Nothing playing",
    play: "Play",
    pause: "Pause",
    next: "Next",
    previous: "Previous",
    show_window: "Show window",
    hide_window: "Hide window",
    close_to_tray: "Keep playing when the window is closed",
    notifications: "Track notifications",
    quit: "Quit",
};

pub const PT: Labels = Labels {
    nothing_playing: "Nada tocando",
    play: "Tocar",
    pause: "Pausar",
    next: "Próxima",
    previous: "Anterior",
    show_window: "Mostrar janela",
    hide_window: "Ocultar janela",
    close_to_tray: "Continuar tocando ao fechar a janela",
    notifications: "Notificações de faixa",
    quit: "Sair",
};

pub fn labels_for(locale: Option<&str>) -> &'static Labels {
    let language = locale
        .unwrap_or_default()
        .split(['-', '_', '.'])
        .next()
        .unwrap_or_default();
    if language.eq_ignore_ascii_case("pt") {
        &PT
    } else {
        &EN
    }
}

pub fn system_labels() -> &'static Labels {
    labels_for(sys_locale::get_locale().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_003_9_portuguese_locales_get_portuguese() {
        for locale in ["pt", "pt-BR", "pt_BR", "pt_PT.UTF-8", "PT-br"] {
            assert_eq!(labels_for(Some(locale)), &PT, "{locale}");
        }
    }

    #[test]
    fn ac_003_9_everything_else_gets_english() {
        for locale in [
            Some("en-US"),
            Some("es-ES"),
            Some("C"),
            Some("pth"),
            Some(""),
            None,
        ] {
            assert_eq!(labels_for(locale), &EN, "{locale:?}");
        }
    }
}
