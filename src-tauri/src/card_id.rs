use rand::Rng;

pub const CARD_ID_LEN: usize = 10;
pub const SHORT_CARD_ID_LEN: usize = 4;

fn random_upper(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let value = rng.sample(rand::distributions::Alphanumeric) as char;
            value.to_ascii_uppercase()
        })
        .collect()
}

pub fn generate_card_id() -> String {
    random_upper(CARD_ID_LEN)
}

pub fn generate_short_card_id() -> String {
    random_upper(SHORT_CARD_ID_LEN)
}

pub fn is_valid_card_id(value: &str) -> bool {
    value.len() == CARD_ID_LEN
        && value
            .chars()
            .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
}

pub fn is_valid_short_card_id(value: &str) -> bool {
    value.len() == SHORT_CARD_ID_LEN
        && value
            .chars()
            .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
}
