pub static WORDLIST: [&str; 256] = [
    "bulbasaur",
    "ivysaur",
    "venusaur",
    "charmander",
    "charmeleon",
    "charizard",
    "squirtle",
    "wartortle",
    "blastoise",
    "caterpie",
    "butterfree",
    "weedle",
    "beedrill",
    "pidgey",
    "pidgeot",
    "rattata",
    "raticate",
    "spearow",
    "fearow",
    "ekans",
    "arbok",
    "pikachu",
    "raichu",
    "sandshrew",
    "sandslash",
    "nidoran",
    "nidorina",
    "nidoqueen",
    "nidorino",
    "nidoking",
    "clefairy",
    "clefable",
    "vulpix",
    "ninetales",
    "jigglypuff",
    "wigglytuff",
    "zubat",
    "golbat",
    "oddish",
    "gloom",
    "vileplume",
    "paras",
    "parasect",
    "venonat",
    "venomoth",
    "diglett",
    "dugtrio",
    "meowth",
    "persian",
    "psyduck",
    "golduck",
    "mankey",
    "primeape",
    "growlithe",
    "arcanine",
    "poliwag",
    "poliwhirl",
    "poliwrath",
    "abra",
    "kadabra",
    "alakazam",
    "machop",
    "machoke",
    "machamp",
    "bellsprout",
    "weepinbell",
    "victreebel",
    "tentacool",
    "tentacruel",
    "geodude",
    "graveler",
    "golem",
    "ponyta",
    "rapidash",
    "slowpoke",
    "slowbro",
    "magnemite",
    "magneton",
    "farfetchd",
    "doduo",
    "dodrio",
    "seel",
    "dewgong",
    "grimer",
    "muk",
    "shellder",
    "cloyster",
    "gastly",
    "haunter",
    "gengar",
    "onix",
    "drowzee",
    "hypno",
    "krabby",
    "kingler",
    "voltorb",
    "electrode",
    "exeggcute",
    "exeggutor",
    "cubone",
    "marowak",
    "hitmonlee",
    "hitmonchan",
    "lickitung",
    "koffing",
    "weezing",
    "rhyhorn",
    "rhydon",
    "chansey",
    "tangela",
    "kangaskhan",
    "horsea",
    "seadra",
    "goldeen",
    "seaking",
    "staryu",
    "starmie",
    "scyther",
    "jynx",
    "electabuzz",
    "magmar",
    "pinsir",
    "tauros",
    "magikarp",
    "gyarados",
    "lapras",
    "ditto",
    "eevee",
    "vaporeon",
    "jolteon",
    "flareon",
    "porygon",
    "omanyte",
    "omastar",
    "kabuto",
    "kabutops",
    "aerodactyl",
    "snorlax",
    "articuno",
    "zapdos",
    "moltres",
    "dratini",
    "dragonair",
    "dragonite",
    "mewtwo",
    "mew",
    "chikorita",
    "bayleef",
    "meganium",
    "cyndaquil",
    "quilava",
    "typhlosion",
    "totodile",
    "croconaw",
    "feraligatr",
    "sentret",
    "furret",
    "hoothoot",
    "noctowl",
    "ledyba",
    "spinarak",
    "ariados",
    "crobat",
    "chinchou",
    "lanturn",
    "pichu",
    "cleffa",
    "igglybuff",
    "togepi",
    "togetic",
    "natu",
    "xatu",
    "mareep",
    "flaaffy",
    "ampharos",
    "bellossom",
    "marill",
    "azumarill",
    "sudowoodo",
    "politoed",
    "hoppip",
    "skiploom",
    "jumpluff",
    "aipom",
    "sunkern",
    "sunflora",
    "yanma",
    "wooper",
    "quagsire",
    "espeon",
    "umbreon",
    "murkrow",
    "slowking",
    "misdreavus",
    "unown",
    "wobbuffet",
    "girafarig",
    "pineco",
    "forretress",
    "dunsparce",
    "gligar",
    "steelix",
    "snubbull",
    "granbull",
    "qwilfish",
    "scizor",
    "shuckle",
    "heracross",
    "sneasel",
    "teddiursa",
    "ursaring",
    "slugma",
    "magcargo",
    "swinub",
    "piloswine",
    "corsola",
    "remoraid",
    "octillery",
    "delibird",
    "mantine",
    "skarmory",
    "houndour",
    "houndoom",
    "kingdra",
    "phanpy",
    "donphan",
    "porygon2",
    "stantler",
    "smeargle",
    "tyrogue",
    "hitmontop",
    "smoochum",
    "elekid",
    "magby",
    "miltank",
    "blissey",
    "raikou",
    "entei",
    "suicune",
    "larvitar",
    "pupitar",
    "tyranitar",
    "lugia",
    "celebi",
    "treecko",
    "grovyle",
    "sceptile",
    "torchic",
    "combusken",
    "blaziken",
    "mudkip",
    "marshtomp",
    "swampert",
    "ralts",
    "gardevoir",
    "absol",
];

pub fn generate_code() -> String {
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    let mut rng = thread_rng();
    let mut indices = Vec::with_capacity(4);

    let mut nums: Vec<usize> = (0..256).collect();
    nums.shuffle(&mut rng);

    for &num in nums.iter().take(4) {
        indices.push(num);
    }

    indices
        .iter()
        .map(|&i| WORDLIST[i])
        .collect::<Vec<_>>()
        .join("-")
}

pub fn is_valid_code(s: &str) -> bool {
    validate_words(s)
}

fn validate_words(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 4 {
        return false;
    }
    let mut seen = Vec::with_capacity(4);
    for word in &parts {
        let lower = word.to_lowercase();
        match WORDLIST.iter().position(|w| *w == lower) {
            Some(idx) => {
                if seen.contains(&idx) {
                    return false;
                }
                seen.push(idx);
            }
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_code() {
        assert!(is_valid_code("bulbasaur-ivysaur-venusaur-charmander"));
        assert!(is_valid_code("BULBASAUR-ivysaur-VENUSAUR-charmander"));
        assert!(is_valid_code("blaziken-mudkip-marshtomp-swampert"));
    }

    #[test]
    fn test_invalid_length() {
        assert!(!is_valid_code(""));
        assert!(!is_valid_code("bulbasaur"));
        assert!(!is_valid_code("bulbasaur-ivysaur"));
        assert!(!is_valid_code("bulbasaur-ivysaur-venusaur"));
        assert!(!is_valid_code(
            "bulbasaur-ivysaur-venusaur-charmander-charmeleon"
        ));
    }

    #[test]
    fn test_invalid_words() {
        assert!(!is_valid_code("bulbasaur-ivysaur-venusaur-agumon"));
        assert!(!is_valid_code("123-ivysaur-venusaur-charmander"));
        assert!(!is_valid_code("bulbasaur-ivysaur-venusaur-"));
    }

    #[test]
    fn test_duplicate_words() {
        assert!(!is_valid_code("bulbasaur-bulbasaur-venusaur-charmander"));
        assert!(!is_valid_code("bulbasaur-ivysaur-venusaur-ivysaur"));
    }

    #[test]
    fn test_generate_code_validity() {
        for _ in 0..100 {
            let code = generate_code();
            assert!(is_valid_code(&code), "Generated code {} is invalid", code);
        }
    }
}
