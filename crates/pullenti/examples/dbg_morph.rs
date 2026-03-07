use pullenti_morph::{MorphologyService, MorphLang};
fn main() {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    for text in &["Howard-Snyder test", "Kevin Hef-fernan", "Ken-ton Lee"] {
        let tokens = MorphologyService::process(text, Some(MorphLang::RU | MorphLang::EN));
        println!("\n{text:?}:");
        if let Some(toks) = &tokens {
            for t in toks {
                println!("  term={:?} begin={} end={}", t.term, t.begin_char, t.end_char);
            }
        }
    }
}
