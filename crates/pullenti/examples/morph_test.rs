use pullenti::pullenti_morph::{MorphologyService, MorphLang};

fn main() {
    MorphologyService::initialize(Some(MorphLang::RU));
    let words = ["разрабатывается", "программистом"];
    for w in &words {
        let tokens = MorphologyService::process(w, None).unwrap_or_default();
        println!("=== {} ===", w);
        for t in tokens.iter() {
            for wf in t.word_forms.iter().flatten() {
                let misc_attrs: Vec<String> = wf.misc.as_ref().map(|m| m.attrs.clone()).unwrap_or_default();
                let voice = wf.misc.as_ref().map(|m| m.voice()).unwrap_or(pullenti::pullenti_morph::MorphVoice::Undefined);
                println!("  nc={:?} nf={:?} case={:?} voice={:?} misc={:?}",
                    wf.normal_case, wf.normal_full, wf.base.case, voice, misc_attrs);
            }
        }
    }
}
