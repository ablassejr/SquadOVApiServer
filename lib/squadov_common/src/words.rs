mod nouns;
mod adjectives;
mod verbs;
use rand::Rng;

pub fn random_noun() -> &'static str {
    random_from_wordlist(nouns::NOUN_LIST)
}

pub fn random_adjective() -> &'static str {
    random_from_wordlist(adjectives::ADJECTIVE_LIST)
}

pub fn random_verb() -> &'static str {
    random_from_wordlist(verbs::VERB_LIST)
}

fn random_from_wordlist(wordlist: &[&'static str]) -> &'static str {
    let mut rng = rand::thread_rng();
    wordlist[rng.gen_range(0..wordlist.len())]
}