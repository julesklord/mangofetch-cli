pub const POKEMON_NAMES: &[&str] = &[
    "bulbasaur",
    "charmander",
    "squirtle",
    "pikachu",
    "jigglypuff",
    "meowth",
    "psyduck",
    "machop",
    "geodude",
    "gengar",
    "eevee",
    "snorlax",
    "mewtwo",
    "mew",
];

pub fn random_pokemon_name() -> &'static str {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;
    POKEMON_NAMES[seed % POKEMON_NAMES.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_pokemon_name_not_empty() {
        let name = random_pokemon_name();
        assert!(!name.is_empty(), "Pokemon name should not be empty");
    }

    #[test]
    fn test_random_pokemon_name_multiple_calls_no_panic() {
        for _ in 0..100 {
            let name = random_pokemon_name();
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn test_random_pokemon_name_returns_valid_name() {
        let name = random_pokemon_name();
        assert!(
            POKEMON_NAMES.contains(&name),
            "Returned name should be in the list of pokemon names"
        );
    }
}
