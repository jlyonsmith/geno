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

/// Returns `true` if the first character of the string is uppercase.
pub fn is_first_char_uppercase(s: &str) -> bool {
    // Get an iterator over the characters and try to get the first one
    match s.chars().next() {
        Some(first_char) => {
            // Use the built-in method to check if the character is uppercase
            first_char.is_uppercase() //
        }
        // If the string is empty, it doesn't have an uppercase first character
        None => false,
    }
}

/// Returns `true` if the first character of the string is lowercase.
pub fn is_first_char_lowercase(s: &str) -> bool {
    // Get an iterator over the characters and try to get the first one
    match s.chars().next() {
        Some(first_char) => {
            // Use the built-in method to check if the character is lowercase
            first_char.is_lowercase() //
        }
        // If the string is empty, it doesn't have a lowercase first character
        None => false,
    }
}
