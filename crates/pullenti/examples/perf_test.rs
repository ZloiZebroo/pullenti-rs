use pullenti_morph::{MorphologyService, MorphLang};
fn main() {
    MorphologyService::initialize(Some(MorphLang::RU));
    let results = MorphologyService::process("Анфиса Константиновна Сапожкова оттого", Some(MorphLang::RU)).unwrap();
    for t in &results {
        println!("Token: term={:?}", t.term);
        if let Some(ref wfs) = t.word_forms {
            for wf in wfs {
                println!("  class={} nc={:?} nf={:?} is_proper_name={} is_proper_surname={}",
                    wf.base.class.value, wf.normal_case, wf.normal_full,
                    wf.base.class.is_proper_name(), wf.base.class.is_proper_surname());
            }
        }
    }
}
