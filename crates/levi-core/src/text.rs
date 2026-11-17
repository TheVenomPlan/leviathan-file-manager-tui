//! Utilidades de texto para búsquedas y filtros.

/// Normaliza un texto para compararlo sin distinguir mayúsculas ni acentos.
///
/// Ejemplos: `"Música"` y `"MUSICA"` producen `"musica"`; `"España"` produce
/// `"espana"`. También elimina marcas diacríticas combinadas (Unicode NFD).
pub fn normalize_search(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .filter_map(fold_char)
        .collect()
}

/// Convierte un carácter acentuado en su equivalente sin acento.
fn fold_char(character: char) -> Option<char> {
    // Marcas diacríticas combinadas (texto Unicode descompuesto).
    if ('\u{0300}'..='\u{036F}').contains(&character) {
        return None;
    }

    Some(match character {
        'á' | 'à' | 'ä' | 'â' | 'ã' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'ç' | 'ć' | 'č' => 'c',
        'ď' | 'đ' => 'd',
        'é' | 'è' | 'ë' | 'ê' | 'ē' | 'ė' | 'ę' | 'ě' => 'e',
        'ğ' => 'g',
        'í' | 'ì' | 'ï' | 'î' | 'ī' | 'į' => 'i',
        'ł' => 'l',
        'ñ' | 'ń' | 'ň' => 'n',
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' | 'ø' | 'ō' | 'ő' => 'o',
        'ř' => 'r',
        'ś' | 'š' => 's',
        'ť' => 't',
        'ú' | 'ù' | 'ü' | 'û' | 'ū' | 'ů' | 'ű' | 'ų' => 'u',
        'ý' | 'ÿ' => 'y',
        'ź' | 'ż' | 'ž' => 'z',
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::normalize_search;

    #[test]
    fn removes_case_and_accents() {
        assert_eq!(normalize_search("Música"), "musica");
        assert_eq!(normalize_search("MÚSICA"), "musica");
        assert_eq!(normalize_search("canción"), "cancion");
        assert_eq!(normalize_search("España"), "espana");
        assert_eq!(normalize_search("Año"), "ano");
        assert_eq!(normalize_search("über"), "uber");
    }

    #[test]
    fn removes_combining_marks() {
        assert_eq!(normalize_search("Mu\u{0301}sica"), "musica");
        assert_eq!(normalize_search("Espan\u{0303}a"), "espana");
    }
}
