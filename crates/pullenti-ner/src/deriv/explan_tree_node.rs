/// ExplanTreeNode — trie node used by DerivateDictionary.
/// Mirrors `ExplanTreeNode.cs`.  We use eager (non-lazy) loading for simplicity.

use std::collections::HashMap;
use pullenti_morph::internal::byte_array_wrapper::ByteArrayWrapper;
use super::deriv_group::DerivateGroup;

#[derive(Default)]
pub struct ExplanTreeNode {
    /// child nodes, keyed by UTF-16 char code (i16)
    pub nodes:  Option<HashMap<i16, Box<ExplanTreeNode>>>,
    /// 1-based indices into DerivateDictionary.all_groups
    pub groups: Option<Vec<usize>>,
}

impl ExplanTreeNode {
    pub fn new() -> Self { Self::default() }

    /// Deserialize this node.  `all_groups` must already be populated (group data
    /// read before calling this).  `_lazy_load` is ignored — we always load eagerly.
    pub fn deserialize(
        &mut self,
        buf:        &ByteArrayWrapper,
        all_groups: &mut Vec<DerivateGroup>,
        pos:        &mut usize,
    ) {
        // ── groups attached to this node ──────────────────────────────────
        let mut cou = buf.deserialize_short(pos) as i32;
        let mut grp_ids: Vec<usize> = Vec::new();
        while cou > 0 {
            cou -= 1;
            let id = buf.deserialize_int(pos) as usize;
            if id > 0 && id <= all_groups.len() {
                let gr = &all_groups[id - 1];
                // Lazy-pos means group wasn't fully deserialised yet
                if gr.lazy_pos > 0 {
                    let lp = gr.lazy_pos;
                    let mut p0 = lp;
                    all_groups[id - 1].deserialize(buf, &mut p0);
                    all_groups[id - 1].lazy_pos = 0;
                }
            }
            grp_ids.push(id);
        }
        if !grp_ids.is_empty() { self.groups = Some(grp_ids); }

        // ── child nodes ───────────────────────────────────────────────────
        let mut ncou = buf.deserialize_short(pos) as i32;
        if ncou == 0 { return; }
        let mut map: HashMap<i16, Box<ExplanTreeNode>> = HashMap::new();
        while ncou > 0 {
            ncou -= 1;
            let ke = buf.deserialize_short(pos);
            let _p1 = buf.deserialize_int(pos); // "end" offset — unused in eager mode
            let mut child = Box::new(ExplanTreeNode::new());
            child.deserialize(buf, all_groups, pos);
            map.entry(ke).or_insert(child);
        }
        self.nodes = Some(map);
    }
}
