/// Converts a string to PascalCase.
/// "type1" -> "Type1", "kiwiFruit" -> "KiwiFruit", "alpha_beta" -> "AlphaBeta"
pub fn to_pascal(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.push_str(chars.as_str());
                    s
                }
            }
        })
        .collect()
}

/// Converts a string to camelCase.
/// "alpha_beta" -> "alphaBeta", "AlphaBeta" -> "alphaBeta"
pub fn to_camel(s: &str) -> String {
    let parts: Vec<&str> = s.split('_').collect();
    let mut result = String::new();

    for (i, part) in parts.iter().enumerate() {
        let mut chars = part.chars();
        match chars.next() {
            None => {}
            Some(c) => {
                if i == 0 {
                    for lc in c.to_lowercase() {
                        result.push(lc);
                    }
                } else {
                    for uc in c.to_uppercase() {
                        result.push(uc);
                    }
                }
                result.push_str(chars.as_str());
            }
        }
    }

    result
}

/// Converts a string to snake_case.
/// "alphaBeta" -> "alpha_beta", "alpha_beta" -> "alpha_beta"
pub fn to_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        for lc in c.to_lowercase() {
            result.push(lc);
        }
    }
    result
}

/// Returns `true` if the name is in PascalCase (first character is uppercase).
pub fn is_pascal_case(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.is_uppercase() && chars.all(|c| c.is_alphanumeric()),
        None => false,
    }
}

/// Returns `true` if the name is in camelCase (first character is lowercase).
pub fn is_camel_case(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.is_lowercase() && chars.all(|c| c.is_alphanumeric()),
        None => false,
    }
}

mod test {
    #[test]
    fn test_is_pascal_case() {
        assert!(super::is_pascal_case("Pascal"));
        assert!(super::is_pascal_case("PascalCase"));
        assert!(!super::is_pascal_case("camelCase"));
        assert!(!super::is_pascal_case("snake_case"));
    }

    #[test]
    fn test_is_camel_case() {
        assert!(super::is_camel_case("camel"));
        assert!(!super::is_camel_case("PascalCase"));
        assert!(super::is_camel_case("camelCase"));
        assert!(!super::is_camel_case("snake_case"));
    }
}
