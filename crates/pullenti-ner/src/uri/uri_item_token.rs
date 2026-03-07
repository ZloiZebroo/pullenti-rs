/// URI parsing primitives — port of UriItemToken.cs

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::token::{TokenRef, TokenKind, NumberSpellingType};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::core::{Termin, TerminCollection};

// ── Standard domain-group TLDs ────────────────────────────────────────────────

const DOMAIN_GROUPS: &[&str] = &[
    "com;net;org;inf;biz;name;aero;arpa;edu;int;gov;mil;coop;museum;mobi;travel",
    "ac;ad;ae;af;ag;ai;al;am;an;ao;aq;ar;as;at;au;aw;az",
    "ba;bb;bd;be;bf;bg;bh;bi;bj;bm;bn;bo;br;bs;bt;bv;bw;by;bz",
    "ca;cc;cd;cf;cg;ch;ci;ck;cl;cm;cn;co;cr;cu;cv;cx;cy;cz",
    "de;dj;dk;dm;do;dz",
    "ec;ee;eg;eh;er;es;et;eu",
    "fi;fj;fk;fm;fo;fr",
    "ga;gd;ge;gf;gg;gh;gi;gl;gm;gn;gp;gq;gr;gs;gt;gu;gw;gy",
    "hk;hm;hn;hr;ht;hu",
    "id;ie;il;im;in;io;iq;ir;is;it",
    "je;jm;jo;jp",
    "ke;kg;kh;ki;km;kn;kp;kr;kw;ky;kz",
    "la;lb;lc;li;lk;lr;ls;lt;lu;lv;ly",
    "ma;mc;md;mg;mh;mk;ml;mm;mn;mo;mp;mq;mr;ms;mt;mu;mv;mw;mx;my;mz",
    "na;nc;ne;nf;ng;ni;nl;no;np;nr;nu;nz",
    "om",
    "pa;pe;pf;pg;ph;pk;pl;pm;pn;pr;ps;pt;pw;py",
    "qa",
    "re;ro;ru;rw",
    "sa;sb;sc;sd;se;sg;sh;si;sj;sk;sl;sm;sn;so;sr;st;su;sv;sy;sz",
    "tc;td;tf;tg;th;tj;tk;tm;tn;to;tp;tr;tt;tv;tw;tz",
    "ua;ug;uk;um;us;uy;uz",
    "va;vc;ve;vg;vi;vn;vu",
    "wf;ws",
    "ye;yt;yu",
    "za;zm;zw",
];

static STD_GROUPS: OnceLock<HashSet<String>> = OnceLock::new();

fn std_groups() -> &'static HashSet<String> {
    STD_GROUPS.get_or_init(|| {
        let mut set = HashSet::new();
        for group in DOMAIN_GROUPS {
            for d in group.split(';') {
                set.insert(d.to_ascii_uppercase());
            }
        }
        set
    })
}

pub fn is_std_group(term: &str) -> bool {
    std_groups().contains(&term.to_ascii_uppercase())
}

static STD_GROUPS_TC: OnceLock<TerminCollection> = OnceLock::new();

pub fn std_groups_tc() -> &'static TerminCollection {
    STD_GROUPS_TC.get_or_init(|| {
        let mut tc = TerminCollection::new();
        for group in DOMAIN_GROUPS {
            for d in group.split(';') {
                tc.add(Termin::new(d));
            }
        }
        tc
    })
}

// ── UriItemToken ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct UriItemToken {
    pub begin_token: TokenRef,
    pub end_token: TokenRef,
    pub value: String,
}

impl UriItemToken {
    fn new(begin: TokenRef, end: TokenRef, value: String) -> Self {
        UriItemToken { begin_token: begin, end_token: end, value }
    }

    pub fn begin_char(&self) -> i32 { self.begin_token.borrow().begin_char }
    pub fn end_char(&self) -> i32 { self.end_token.borrow().end_char }

    /// Returns true if upcoming tokens (starting at `t`) eventually reach a domain group extension.
    fn has_domain_group_ahead(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
        let mut cur = t.clone();
        loop {
            let (is_dot, is_hyp, is_ws, is_nl, prev_doh, term, is_lat, next) = {
                let tb = cur.borrow();
                let is_dot = tb.is_char('.', sofa);
                let is_hyp = tb.is_hiphen(sofa);
                let is_ws = tb.is_whitespace_before(sofa) && !std::rc::Rc::ptr_eq(&cur, t);
                let is_nl = tb.is_newline_before(sofa);
                let prev_doh = tb.prev.as_ref().and_then(|w| w.upgrade())
                    .map(|p| { let pb = p.borrow(); pb.is_char('.', sofa) || pb.is_hiphen(sofa) })
                    .unwrap_or(false);
                let term = tb.term().map(|s| s.to_string());
                let is_lat = tb.chars.is_latin_letter();
                let next = tb.next.clone();
                (is_dot, is_hyp, is_ws, is_nl, prev_doh, term, is_lat, next)
            };
            if is_dot || is_hyp {
                cur = match next { None => return false, Some(n) => n };
                continue;
            }
            if is_ws {
                if is_nl { return false; }
                if !prev_doh { return false; }
            }
            match &term {
                None => return false,
                Some(t) => { if is_std_group(t) { return true; } }
            }
            if !is_lat { return false; }
            cur = match next { None => return false, Some(n) => n };
        }
    }

    /// Returns true if content after whitespace looks like a URL path
    fn has_web_ext_ahead(t: &TokenRef, sofa: &SourceOfAnalysis, chars: &str) -> bool {
        let mut cur = t.clone();
        // optionally skip a leading slash
        if cur.borrow().is_char_of("\\/", sofa) {
            let next = cur.borrow().next.clone();
            cur = match next { None => return false, Some(n) => n };
        }
        let base = cur.clone();
        loop {
            let (is_ws, is_num, is_letter, is_lat, term, is_slash, in_chars, next) = {
                let tb = cur.borrow();
                let is_ws = !std::rc::Rc::ptr_eq(&cur, &base) && tb.is_whitespace_before(sofa);
                let is_num = matches!(&tb.kind, TokenKind::Number(_));
                let is_let = tb.is_letters();
                let is_lat = tb.chars.is_latin_letter();
                let term = tb.term().map(|s| s.to_string());
                let is_slash = tb.is_char_of("\\/", sofa);
                let in_chars = term.as_deref().map_or(false, |s| {
                    s.chars().next().map_or(false, |c| chars.contains(c))
                });
                let next = tb.next.clone();
                (is_ws, is_num, is_let, is_lat, term, is_slash, in_chars, next)
            };
            if is_ws { break; }
            if is_num { cur = match next { None => break, Some(n) => n }; continue; }
            if is_letter {
                match &term {
                    None => break,
                    Some(t) => {
                        if ["HTM","HTML","SHTML","ASP","ASPX","JSP"].contains(&t.as_str()) {
                            return true;
                        }
                        if !is_lat { break; }
                    }
                }
            } else {
                if is_slash { return true; }
                if !in_chars { break; }
            }
            cur = match next { None => break, Some(n) => n };
        }
        false
    }
}

// ── Helper: step previous ─────────────────────────────────────────────────────

fn prev_of(t: &TokenRef) -> Option<TokenRef> {
    t.borrow().prev.as_ref().and_then(|w| w.upgrade())
}
fn next_of(t: &TokenRef) -> Option<TokenRef> {
    t.borrow().next.clone()
}

// ── attach_domain_name ────────────────────────────────────────────────────────

pub fn attach_domain_name(
    t0: &TokenRef,
    sofa: &SourceOfAnalysis,
    check: bool,
    can_be_whitespaces: bool,
) -> Option<UriItemToken> {
    let mut txt = String::new();
    let mut t1 = t0.clone();
    let mut ip_count = 0i32;
    let mut is_ip = true;

    let mut t = t0.clone();
    loop {
        let is_first = std::rc::Rc::ptr_eq(&t, t0);
        if !is_first {
            let (ws, nl) = { let tb = t.borrow(); (tb.is_whitespace_before(sofa), tb.is_newline_before(sofa)) };
            if ws {
                let ok = !nl && can_be_whitespaces && UriItemToken::has_domain_group_ahead(&t, sofa);
                if !ok { break; }
            }
        }

        let (is_num, is_text, term_s, is_letter, next) = {
            let tb = t.borrow();
            (
                matches!(&tb.kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit),
                matches!(&tb.kind, TokenKind::Text(_)),
                tb.term().map(|s| s.to_string()),
                tb.chars.is_letter(),
                tb.next.clone(),
            )
        };

        if is_num {
            let nv = { t.borrow().number_value().unwrap_or("").to_string() };
            if let Ok(v) = nv.parse::<u32>() { if v < 256 { ip_count += 1; } else { is_ip = false; } } else { is_ip = false; }
            txt.push_str(&nv);
            t1 = t.clone();
            t = match next { None => break, Some(n) => n };
            continue;
        }
        if !is_text { break; }
        let s = match term_s { None => break, Some(s) => s };
        let ch = s.chars().next().unwrap_or('\0');
        if !is_letter {
            if !".-_".contains(ch) { break; }
            if ch != '.' { is_ip = false; }
            if ch == '-' && txt.to_ascii_lowercase() == "vk.com" {
                return Some(UriItemToken::new(t0.clone(), t1.clone(), txt.to_ascii_lowercase()));
            }
        } else { is_ip = false; }

        txt.push_str(&s.to_ascii_lowercase());
        t1 = t.clone();
        t = match next { None => break, Some(n) => n };
    }

    if txt.is_empty() { return None; }
    if ip_count != 4 { is_ip = false; }

    // Validate / trim trailing dot
    let mut points = 0i32;
    let bytes: Vec<char> = txt.chars().collect();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if bytes[i] == '.' {
            if i == 0 { return None; }
            if i == n - 1 {
                txt.pop();
                let prev_t1 = prev_of(&t1).unwrap_or_else(|| t1.clone());
                t1 = prev_t1;
                break;
            }
            if (i > 0 && bytes[i-1] == '.') || (i+1 < n && bytes[i+1] == '.') { return None; }
            points += 1;
        }
        i += 1;
    }
    if points == 0 { return None; }

    if check {
        let mut ok = is_ip || txt == "localhost";
        if !ok {
            let prev_is_dot = prev_of(&t1).map_or(false, |p| p.borrow().is_char('.', sofa));
            if prev_is_dot {
                if let Some(term) = t1.borrow().term() { if is_std_group(term) { ok = true; } }
            }
        }
        if !ok { return None; }
    }

    Some(UriItemToken::new(t0.clone(), t1, txt))
}

// ── _attach_uri_content (internal) ───────────────────────────────────────────

fn attach_uri_content_inner(
    t0: &TokenRef,
    sofa: &SourceOfAnalysis,
    chars: &str,
    can_be_whitespaces: bool,
) -> Option<UriItemToken> {
    let mut txt = String::new();
    let mut t1 = t0.clone();

    let dom = attach_domain_name(t0, sofa, true, can_be_whitespaces);
    if let Some(ref d) = dom {
        if d.value.len() < 3 { return None; }
    }

    let mut open_char = '\0';
    let start_after_dom: Option<TokenRef> = dom.as_ref().and_then(|d| next_of(&d.end_token));

    let mut t = match dom.as_ref() {
        Some(_) => match start_after_dom { None => return dom.clone(), Some(n) => n },
        None => t0.clone(),
    };

    loop {
        let is_first = std::rc::Rc::ptr_eq(&t, t0);

        if !is_first {
            let (ws, nl) = { let tb = t.borrow(); (tb.is_whitespace_before(sofa), tb.is_newline_before(sofa)) };
            if ws {
                if nl || !can_be_whitespaces { break; }
                if dom.is_none() { break; }

                let prev_ref = prev_of(&t);
                let prev_hyp = prev_ref.as_ref().map_or(false, |p| p.borrow().is_hiphen(sofa));
                let prev_semi = prev_ref.as_ref().map_or(false, |p| p.borrow().is_char_of(",;", sofa));
                let prev_dot = prev_ref.as_ref().map_or(false, |p| p.borrow().is_char('.', sofa));
                let (t_let, t_len) = { let tb = t.borrow(); (tb.chars.is_letter(), tb.length_char()) };

                if prev_hyp {
                    // ok, continue
                } else if prev_semi {
                    break;
                } else if prev_dot && t_let && t_len == 2 {
                    // abbreviated word — continue
                } else {
                    if !UriItemToken::has_web_ext_ahead(&t, sofa, chars) { break; }
                }
            }
        }

        // Number token
        let is_num = { matches!(&t.borrow().kind, TokenKind::Number(_)) };
        if is_num {
            let src = { t.borrow().get_source_text(sofa).to_string() };
            txt.push_str(&src);
            t1 = t.clone();
            t = match next_of(&t) { None => break, Some(n) => n };
            continue;
        }

        // Referent token
        let ref_opt = t.borrow().get_referent();
        if let Some(_r) = ref_opt {
            let is_rf_val = t.borrow().is_value("РФ", None);
            if is_rf_val && !txt.is_empty() && txt.ends_with('.') {
                let src = { t.borrow().get_source_text(sofa).to_string() };
                txt.push_str(&src);
                t1 = t.clone();
                t = match next_of(&t) { None => break, Some(n) => n };
                continue;
            }
            let (is_lat, is_single) = {
                let tb = t.borrow();
                let lat = tb.chars.is_latin_letter();
                let single = match &tb.kind {
                    TokenKind::Referent(r) => {
                        let bc = r.meta.begin_token.as_ref().map_or(-1, |b| b.borrow().begin_char);
                        let ec = r.meta.end_token.as_ref().map_or(-1, |e| e.borrow().end_char);
                        bc == ec || bc == -1
                    }
                    _ => false,
                };
                (lat, single)
            };
            if is_lat && is_single {
                let src = { t.borrow().get_source_text(sofa).to_string() };
                txt.push_str(&src);
                t1 = t.clone();
                t = match next_of(&t) { None => break, Some(n) => n };
                continue;
            }
            break;
        }

        // Text token
        let (is_text, src_opt) = {
            let tb = t.borrow();
            let is_text = matches!(&tb.kind, TokenKind::Text(_));
            let src = if is_text { Some(tb.get_source_text(sofa).to_string()) } else { None };
            (is_text, src)
        };
        if !is_text { break; }
        let src = src_opt.unwrap();
        let ch = src.chars().next().unwrap_or('\0');

        if !ch.is_alphabetic() {
            if !chars.contains(ch) { break; }
            if ch == '(' || ch == '[' { open_char = ch; }
            else if ch == ')' { if open_char != '(' { break; } open_char = '\0'; }
            else if ch == ']' { if open_char != '[' { break; } open_char = '\0'; }
        }

        txt.push_str(&src);
        t1 = t.clone();
        t = match next_of(&t) { None => break, Some(n) => n };
    }

    if txt.is_empty() { return dom; }
    if !txt.chars().any(|c| c.is_alphanumeric()) { return dom; }

    // Trim trailing dot or slash
    if txt.ends_with('.') || txt.ends_with('/') {
        txt.pop();
        let new_t1 = prev_of(&t1).unwrap_or_else(|| t1.clone());
        t1 = new_t1;
    }

    if let Some(ref d) = dom {
        txt.insert_str(0, &d.value);
    }

    if txt.starts_with("\\\\") {
        let rest = txt[2..].to_string();
        txt = format!("//{}", rest);
    }
    let result_val = if txt.starts_with("//") { txt[2..].to_string() } else { txt.clone() };
    if result_val.eq_ignore_ascii_case("WWW") { return None; }

    Some(UriItemToken::new(t0.clone(), t1, txt))
}

// ── Public: attach_uri_content ────────────────────────────────────────────────

pub fn attach_uri_content(
    t0: &TokenRef,
    sofa: &SourceOfAnalysis,
    after_http: bool,
) -> Option<UriItemToken> {
    let mut res = attach_uri_content_inner(t0, sofa, ".;:-_=+&%#@/\\?[]()!~", after_http)?;

    // Trim trailing separators
    while res.value.ends_with(|c| ".;-:".contains(c)) && res.end_char() > 3 {
        res.value.pop();
        let new_end = prev_of(&res.end_token).unwrap_or_else(|| res.end_token.clone());
        res.end_token = new_end;
    }
    if res.value.ends_with('/') {
        res.value.pop();
        let new_end = prev_of(&res.end_token).unwrap_or_else(|| res.end_token.clone());
        res.end_token = new_end;
    }
    if res.value.ends_with('\\') {
        res.value.pop();
        let new_end = prev_of(&res.end_token).unwrap_or_else(|| res.end_token.clone());
        res.end_token = new_end;
    }
    if res.value.contains('\\') { res.value = res.value.replace('\\', "/"); }
    if res.value.is_empty() { return None; }
    Some(res)
}

// ── attach_url ────────────────────────────────────────────────────────────────

pub fn attach_url(t0: &TokenRef, sofa: &SourceOfAnalysis) -> Option<UriItemToken> {
    let srv = attach_domain_name(t0, sofa, true, false)?;
    let mut txt = srv.value.clone();
    let mut t1 = srv.end_token.clone();

    // Optional port: :NUMBER
    {
        let n1 = next_of(&t1);
        if let Some(n) = n1 {
            if n.borrow().is_char(':', sofa) {
                if let Some(nn) = next_of(&n) {
                    if matches!(&nn.borrow().kind, TokenKind::Number(_)) {
                        let pv = { nn.borrow().number_value().unwrap_or("").to_string() };
                        txt.push(':');
                        txt.push_str(&pv);
                        t1 = nn;
                    }
                }
            }
        }
    }

    // vk.com + hyphen special case
    {
        let is_vk = txt == "vk.com";
        let n1 = next_of(&t1);
        if is_vk {
            if let Some(n) = n1 {
                if n.borrow().is_hiphen(sofa) {
                    if let Some(n2) = next_of(&n) {
                        if let Some(dat) = attach_uri_content_inner(&n2, sofa, ".-_+%", false) {
                            t1 = dat.end_token.clone();
                            txt.push('/');
                            txt.push_str(&dat.value);
                        }
                    }
                }
            }
        }
    }

    // Path segments
    loop {
        let n1 = next_of(&t1);
        let slash = match n1 { None => break, Some(n) => n };
        if !slash.borrow().is_char('/', sofa) { break; }
        if slash.borrow().is_whitespace_after(sofa) {
            let new_t1 = slash.clone();
            t1 = new_t1;
            break;
        }
        let after = match next_of(&slash) {
            None => { t1 = slash; break; }
            Some(n) => n,
        };
        match attach_uri_content_inner(&after, sofa, ".-_+%", false) {
            None => { t1 = slash; break; }
            Some(d) => {
                t1 = d.end_token.clone();
                txt.push('/');
                txt.push_str(&d.value);
            }
        }
    }

    // Query string ?
    {
        let n1 = next_of(&t1);
        if let Some(n) = n1 {
            let ok = n.borrow().is_char('?', sofa)
                && !n.borrow().is_whitespace_after(sofa)
                && !t1.borrow().is_whitespace_after(sofa);
            if ok {
                if let Some(aq) = next_of(&n) {
                    if let Some(d) = attach_uri_content_inner(&aq, sofa, ".-_+%=&", false) {
                        t1 = d.end_token.clone();
                        txt.push('?');
                        txt.push_str(&d.value);
                    }
                }
            }
        }
    }

    // Fragment #
    {
        let n1 = next_of(&t1);
        if let Some(n) = n1 {
            let ok = n.borrow().is_char('#', sofa)
                && !n.borrow().is_whitespace_after(sofa)
                && !t1.borrow().is_whitespace_after(sofa);
            if ok {
                if let Some(ah) = next_of(&n) {
                    if let Some(d) = attach_uri_content_inner(&ah, sofa, ".-_+%", false) {
                        t1 = d.end_token.clone();
                        txt.push('#');
                        txt.push_str(&d.value);
                    }
                }
            }
        }
    }

    if !txt.chars().any(|c| c.is_alphabetic()) { return None; }
    Some(UriItemToken::new(t0.clone(), t1, txt))
}

// ── attach_iso_content ────────────────────────────────────────────────────────

pub fn attach_iso_content(
    t0: &TokenRef,
    sofa: &SourceOfAnalysis,
    spec_chars: &str,
) -> Option<UriItemToken> {
    let mut t = t0.clone();
    // Skip leading separators and IEC
    loop {
        let (is_sep, is_iec, next) = {
            let tb = t.borrow();
            (tb.is_char_of(":/\\", sofa) || tb.is_hiphen(sofa), tb.is_value("IEC", None), tb.next.clone())
        };
        if is_sep || is_iec { t = match next { None => return None, Some(n) => n }; continue; }
        break;
    }
    if !matches!(&t.borrow().kind, TokenKind::Number(_)) { return None; }

    let t_start = t.clone();
    let mut t1 = t.clone();
    let mut delim = '\0';
    let mut txt = String::new();

    loop {
        let (is_ws, is_num, is_text, src, first_ch, in_spec, next) = {
            let tb = t.borrow();
            let is_ws = tb.is_whitespace_before(sofa) && !std::rc::Rc::ptr_eq(&t, &t_start);
            let is_num = matches!(&tb.kind, TokenKind::Number(_));
            let is_text = matches!(&tb.kind, TokenKind::Text(_));
            let src = tb.get_source_text(sofa).to_string();
            let fc = src.chars().next().unwrap_or('\0');
            let in_s = is_text && spec_chars.contains(fc);
            let next = tb.next.clone();
            (is_ws, is_num, is_text, src, fc, in_s, next)
        };
        if is_ws { break; }
        if is_num {
            if delim != '\0' { txt.push(delim); delim = '\0'; }
            txt.push_str(&src);
            t1 = t.clone();
            t = match next { None => break, Some(n) => n };
            continue;
        }
        if is_text && in_spec { delim = first_ch; t = match next { None => break, Some(n) => n }; continue; }
        break;
    }
    if txt.is_empty() { return None; }
    Some(UriItemToken::new(t0.clone(), t1, txt))
}

// ── attach_isbn ───────────────────────────────────────────────────────────────

pub fn attach_isbn(t0: &TokenRef, sofa: &SourceOfAnalysis) -> Option<UriItemToken> {
    let mut txt = String::new();
    let mut t1 = t0.clone();
    let mut digs = 0usize;

    let mut t = t0.clone();
    loop {
        let (is_tc, is_nl, is_num, term, src, next) = {
            let tb = t.borrow();
            let is_tc = tb.is_table_control_char(sofa);
            let is_nl = tb.is_newline_before(sofa) && !std::rc::Rc::ptr_eq(&t, t0);
            let is_num = matches!(&tb.kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit);
            let term = tb.term().map(|s| s.to_string());
            let src = tb.get_source_text(sofa).to_string();
            let next = tb.next.clone();
            (is_tc, is_nl, is_num, term, src, next)
        };
        if is_tc { break; }
        if is_nl {
            let prev_hyp = prev_of(&t).map_or(false, |p| p.borrow().is_hiphen(sofa));
            if !prev_hyp { break; }
        }
        if is_num {
            txt.push_str(&src);
            digs += src.len();
            t1 = t.clone();
            if digs > 13 { break; }
            t = match next { None => break, Some(n) => n };
            continue;
        }
        let s = match term { None => break, Some(s) => s };
        if s != "-" && s != "Х" && s != "X" { break; }
        let out = if s == "Х" { "X" } else { &s };
        txt.push_str(out);
        t1 = t.clone();
        if s != "-" { break; }
        t = match next { None => break, Some(n) => n };
    }
    let dig_count = txt.chars().filter(|c| c.is_ascii_digit()).count();
    if dig_count < 7 { return None; }
    Some(UriItemToken::new(t0.clone(), t1, txt))
}

// ── attach_bbk ───────────────────────────────────────────────────────────────

pub fn attach_bbk(t0: &TokenRef, sofa: &SourceOfAnalysis) -> Option<UriItemToken> {
    let mut txt = String::new();
    let mut t1 = t0.clone();
    let mut digs = 0usize;

    let mut t = t0.clone();
    loop {
        let (is_nl, is_tc, is_num, is_text, src, first_ch, is_ws_before, next) = {
            let tb = t.borrow();
            let is_nl = tb.is_newline_before(sofa) && !std::rc::Rc::ptr_eq(&t, t0);
            let is_tc = tb.is_table_control_char(sofa);
            let is_num = matches!(&tb.kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit);
            let is_text = matches!(&tb.kind, TokenKind::Text(_));
            let src = tb.get_source_text(sofa).to_string();
            let fc = src.chars().next().unwrap_or('\0');
            let ws = tb.is_whitespace_before(sofa);
            let next = tb.next.clone();
            (is_nl, is_tc, is_num, is_text, src, fc, ws, next)
        };
        if is_nl || is_tc { break; }
        if is_num {
            txt.push_str(&src);
            digs += src.len();
            t1 = t.clone();
            t = match next { None => break, Some(n) => n };
            continue;
        }
        if !is_text { break; }
        if first_ch == ',' { break; }
        if first_ch == '(' {
            let nxt_is_num = next.as_ref().map_or(false, |n| matches!(&n.borrow().kind, TokenKind::Number(_)));
            if !nxt_is_num { break; }
        }
        if first_ch.is_alphabetic() && is_ws_before { break; }
        txt.push_str(&src);
        t1 = t.clone();
        t = match next { None => break, Some(n) => n };
    }
    if txt.len() < 3 || digs < 2 { return None; }
    if txt.ends_with('.') {
        txt.pop();
        let new_t1 = prev_of(&t1).unwrap_or_else(|| t1.clone());
        t1 = new_t1;
    }
    Some(UriItemToken::new(t0.clone(), t1, txt))
}

// ── attach_skype ──────────────────────────────────────────────────────────────

pub fn attach_skype(t0: &TokenRef, sofa: &SourceOfAnalysis, tlg: bool) -> Option<UriItemToken> {
    if t0.borrow().chars.is_cyrillic_letter() && !tlg { return None; }

    let mut start = t0.clone();
    if tlg && t0.borrow().is_char('@', sofa) {
        start = next_of(t0)?;
    }

    let res = attach_uri_content_inner(&start, sofa, "._", false)?;

    if tlg {
        let res_lower = res.value.to_ascii_lowercase();
        if res_lower == "http" || res_lower == "https" {
            let mut tt = next_of(&res.end_token);
            if let Some(ref n) = tt.clone() {
                if n.borrow().is_char(':', sofa) {
                    tt = next_of(n);
                    loop {
                        let n2 = match tt.clone() { None => break, Some(x) => x };
                        if n2.borrow().is_char_of("\\/", sofa) { tt = next_of(&n2); } else { break; }
                    }
                    if let Some(ref n3) = tt { return attach_skype(n3, sofa, true); }
                }
            }
        }
        if res_lower == "t.me" {
            let next = next_of(&res.end_token);
            if let Some(n) = next {
                if n.borrow().is_char_of("\\/", sofa) {
                    if let Some(a) = next_of(&n) {
                        if let Some(mut res1) = attach_uri_content_inner(&a, sofa, "._", false) {
                            res1.begin_token = t0.clone();
                            return Some(res1);
                        }
                    }
                }
            }
        }
    }

    if res.value.len() < 4 { return None; }
    Some(res)
}

// ── attach_icq_content ────────────────────────────────────────────────────────

pub fn attach_icq_content(t0: &TokenRef, sofa: &SourceOfAnalysis) -> Option<UriItemToken> {
    if !matches!(&t0.borrow().kind, TokenKind::Number(_)) { return None; }
    let mut res = attach_isbn(t0, sofa)?;
    res.value = res.value.replace('-', "");
    if !res.value.chars().all(|c| c.is_ascii_digit()) { return None; }
    let len = res.value.len();
    if len < 6 || len > 10 { return None; }
    Some(res)
}

// ── attach_mail_users ─────────────────────────────────────────────────────────

pub fn attach_mail_users(t1: &TokenRef, sofa: &SourceOfAnalysis) -> Option<Vec<UriItemToken>> {
    if t1.borrow().is_char('}', sofa) {
        let inner = prev_of(t1)?;
        let mut res0 = attach_mail_users(&inner, sofa)?;
        let mut cur = prev_of(&res0[0].begin_token);
        loop {
            let c = match cur.take() { None => return None, Some(x) => x };
            if c.borrow().is_char('{', sofa) { res0[0].begin_token = c; return Some(res0); }
            if c.borrow().is_char_of(";,", sofa) { cur = prev_of(&c); continue; }
            let sub = attach_mail_users(&c, sofa)?;
            let first_begin = sub[0].begin_token.clone();
            res0.insert(0, sub.into_iter().next().unwrap());
            cur = prev_of(&first_begin);
        }
    }

    let mut txt = String::new();
    let mut t0 = t1.clone();
    let mut t = t1.clone();

    loop {
        let (is_ws_after, is_num, is_text, src, first_ch, prev_t) = {
            let tb = t.borrow();
            let ws_after = tb.is_whitespace_after(sofa);
            let is_num = matches!(&tb.kind, TokenKind::Number(_));
            let is_text = matches!(&tb.kind, TokenKind::Text(_));
            let src = tb.get_source_text(sofa).to_string();
            let fc = src.chars().next().unwrap_or('\0');
            let prev = tb.prev.as_ref().and_then(|w| w.upgrade());
            (ws_after, is_num, is_text, src, fc, prev)
        };
        if is_ws_after { break; }
        if is_num {
            txt.insert_str(0, &src);
            t0 = t.clone();
            t = match prev_t { None => break, Some(p) => p };
            continue;
        }
        if !is_text { break; }
        if !first_ch.is_alphabetic() && !".-_".contains(first_ch) { break; }
        txt.insert_str(0, &src);
        t0 = t.clone();
        t = match prev_t { None => break, Some(p) => p };
    }

    if txt.is_empty() { return None; }
    Some(vec![UriItemToken::new(t0, t1.clone(), txt.to_ascii_lowercase())])
}
