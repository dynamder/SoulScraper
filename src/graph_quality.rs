use std::collections::{HashMap, HashSet, VecDeque};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::data_model::extractor::{GraphLink, GraphNode};
use crate::data_model::soul_mem::{
    proc::{Action, ActionType, ProcMemLink, ProcMemory},
    sit::{AbstractSituation, SituationType},
    MemoryLinkType, MemoryType,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphQualityReport {
    pub node_count: usize,
    pub edge_count: usize,
    pub node_types: NodeTypeCount,
    pub link_types: LinkTypeCount,
    pub connected_components: usize,
    pub largest_component: usize,
    pub isolated_nodes: usize,
    pub mst_forest_edges: usize,
    pub global_redundancy: f64,
    pub avg_clustering: f64,
    pub community_modularity: f64,
    pub intra_community_ratio: f64,
    pub gini_coefficient: f64,
    pub has_self_node: bool,
    pub self_description_ok: bool,
    pub illegal_edges: Vec<IllegalEdgeEntry>,
    pub is_clean: bool,
    pub is_structurally_valid: bool,
    pub failures: Vec<String>,
    pub warnings: Vec<String>,
    pub has_proc_none: bool,
    pub abstract_sit_type_count: usize,
    pub proc_without_incoming_proc: Vec<String>,
    pub abs_sit_without_proc: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NodeTypeCount {
    pub semantic: usize,
    pub situation: usize,
    pub procedure: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LinkTypeCount {
    pub sem: usize,
    pub proc: usize,
    pub situation: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IllegalEdgeEntry {
    pub from_id: String,
    pub from_type: String,
    pub to_id: String,
    pub to_type: String,
    pub link_type: String,
    pub reason: String,
}

fn classify_node_type(mem_type: &MemoryType) -> &'static str {
    match mem_type {
        MemoryType::Semantic(_) => "Semantic",
        MemoryType::Situation(_) => "Situation",
        MemoryType::Procedure(_) => "Procedure",
    }
}

fn classify_link_type(link_type: &MemoryLinkType) -> &'static str {
    match link_type {
        MemoryLinkType::Sem(_) => "Sem",
        MemoryLinkType::Proc(_) => "Proc",
        MemoryLinkType::Situation(_) => "Situation",
    }
}

fn is_edge_legal(from_type: &str, to_type: &str) -> bool {
    match (from_type, to_type) {
        ("Procedure", "Situation") => false,
        ("Procedure", "Procedure") => false,
        _ => true,
    }
}

fn illegality_reason(from_type: &str, to_type: &str) -> &'static str {
    match (from_type, to_type) {
        ("Procedure", "Situation") => "Procedure -> Situation: behavior cannot trigger event",
        ("Procedure", "Procedure") => {
            "Procedure -> Procedure: procedure does not chain into another"
        }
        _ => "",
    }
}

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }
    fn union(&mut self, x: usize, y: usize) -> bool {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return false;
        }
        if self.rank[rx] < self.rank[ry] {
            self.parent[rx] = ry;
        } else if self.rank[rx] > self.rank[ry] {
            self.parent[ry] = rx;
        } else {
            self.parent[ry] = rx;
            self.rank[rx] += 1;
        }
        true
    }
}

pub fn validate_graph(nodes: &[GraphNode]) -> GraphQualityReport {
    let n = nodes.len();
    let mut id_to_idx: HashMap<&str, usize> = HashMap::new();
    let mut idx_to_type: Vec<&str> = vec!["Unknown"; n];
    for (i, node) in nodes.iter().enumerate() {
        id_to_idx.insert(&node.id, i);
        idx_to_type[i] = classify_node_type(&node.mem_type);
    }

    let mut node_types = NodeTypeCount {
        semantic: 0,
        situation: 0,
        procedure: 0,
    };
    for &t in &idx_to_type {
        match t {
            "Semantic" => node_types.semantic += 1,
            "Situation" => node_types.situation += 1,
            "Procedure" => node_types.procedure += 1,
            _ => {}
        }
    }

    struct EdgeData {
        from_idx: usize,
        to_idx: usize,
        #[allow(dead_code)]
        from_type: String,
        #[allow(dead_code)]
        to_type: String,
        #[allow(dead_code)]
        link_type: String,
        intensity: f64,
    }
    let mut edges: Vec<EdgeData> = Vec::new();
    let mut link_types = LinkTypeCount {
        sem: 0,
        proc: 0,
        situation: 0,
    };
    let mut illegal_edges: Vec<IllegalEdgeEntry> = Vec::new();

    for node in nodes {
        let from_idx = id_to_idx[node.id.as_str()];
        let from_type = idx_to_type[from_idx].to_string();
        for link in &node.mem_links {
            let to_idx = match id_to_idx.get(link.to.as_str()) {
                Some(&idx) => idx,
                None => continue,
            };
            let to_type = idx_to_type[to_idx].to_string();
            let lt = classify_link_type(&link.link_type).to_string();

            match lt.as_str() {
                "Sem" => link_types.sem += 1,
                "Proc" => link_types.proc += 1,
                "Situation" => link_types.situation += 1,
                _ => {}
            }

            if !is_edge_legal(&from_type, &to_type) {
                illegal_edges.push(IllegalEdgeEntry {
                    from_id: node.id.clone(),
                    from_type: from_type.clone(),
                    to_id: link.to.clone(),
                    to_type: to_type.clone(),
                    link_type: lt.clone(),
                    reason: illegality_reason(&from_type, &to_type).to_string(),
                });
            }

            edges.push(EdgeData {
                from_idx,
                to_idx,
                from_type: from_type.clone(),
                to_type,
                link_type: lt,
                intensity: link.intensity,
            });
        }
    }

    let m = edges.len();
    let mut adj: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    for e in &edges {
        adj[e.from_idx].insert(e.to_idx);
        adj[e.to_idx].insert(e.from_idx);
    }

    // ── BFS connected components ──
    let mut visited = vec![false; n];
    let mut component_sizes: Vec<usize> = Vec::new();
    let mut node_to_component: Vec<usize> = vec![0; n];
    for start in 0..n {
        if visited[start] {
            continue;
        }
        let cid = component_sizes.len();
        let mut size = 0;
        let mut q = VecDeque::new();
        q.push_back(start);
        visited[start] = true;
        while let Some(v) = q.pop_front() {
            node_to_component[v] = cid;
            size += 1;
            for &u in &adj[v] {
                if !visited[u] {
                    visited[u] = true;
                    q.push_back(u);
                }
            }
        }
        component_sizes.push(size);
    }
    let components = component_sizes.len();
    let largest = component_sizes.iter().copied().max().unwrap_or(0);
    let isolated = component_sizes.iter().filter(|&&s| s == 1).count();

    // ── Per-component edge count ──
    let mut comp_edges: Vec<usize> = vec![0; components];
    for e in &edges {
        let cid = node_to_component[e.from_idx];
        comp_edges[cid] += 1;
    }

    // ── MST/MSF Kruskal ──
    let mut el: Vec<(usize, usize, f64)> = edges
        .iter()
        .map(|e| (e.from_idx, e.to_idx, 1.0 - e.intensity))
        .collect();
    el.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    let mut uf = UnionFind::new(n);
    let mut msf = 0usize;
    for (u, v, _) in &el {
        if uf.union(*u, *v) {
            msf += 1;
        }
    }
    let global_redundancy = if msf > 0 {
        m as f64 / msf as f64 - 1.0
    } else {
        0.0
    };

    // ── Per-component redundancy (for large enough components) ──
    let large_comps: Vec<usize> = component_sizes
        .iter()
        .enumerate()
        .filter(|(_, sz)| **sz > 3)
        .map(|(cid, _)| cid)
        .collect();
    let min_comp_redundancy = if large_comps.is_empty() {
        f64::MAX
    } else {
        large_comps
            .iter()
            .map(|&cid| {
                let sz = component_sizes[cid];
                let ec = comp_edges[cid];
                if sz > 1 {
                    ec as f64 / (sz - 1) as f64 - 1.0
                } else {
                    0.0
                }
            })
            .fold(f64::MAX, |a, b| a.min(b))
    };

    // ── Average clustering coefficient ──
    let mut tc = 0.0;
    let mut cc = 0usize;
    for v in 0..n {
        let nb: Vec<usize> = adj[v].iter().copied().collect();
        let k = nb.len();
        if k < 2 {
            continue;
        }
        let mut tri = 0usize;
        for i in 0..k {
            for j in (i + 1)..k {
                if adj[nb[i]].contains(&nb[j]) {
                    tri += 1;
                }
            }
        }
        tc += tri as f64 / (k * (k - 1) / 2) as f64;
        cc += 1;
    }
    let avg_clustering = if cc > 0 { tc / cc as f64 } else { 0.0 };

    // ── Label propagation ──
    let mut community: Vec<usize> = node_to_component.clone();
    for _ in 0..30 {
        let mut changed = false;
        for v in 0..n {
            let mut freq: HashMap<usize, usize> = HashMap::new();
            for &u in &adj[v] {
                *freq.entry(community[u]).or_insert(0) += 1;
            }
            if let Some((&best, _)) = freq.iter().max_by_key(|&(_, cnt)| cnt) {
                if community[v] != best {
                    community[v] = best;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    // ── Modularity ──
    let mf = m as f64;
    let mut modularity = 0.0;
    if m > 0 && n > 0 {
        let mut degrees = vec![0.0; n];
        for e in &edges {
            degrees[e.from_idx] += 1.0;
            degrees[e.to_idx] += 1.0;
        }
        let mut q = 0.0;
        for e in &edges {
            if community[e.from_idx] == community[e.to_idx] {
                q += 1.0 - (degrees[e.from_idx] * degrees[e.to_idx]) / (2.0 * mf);
            }
        }
        modularity = q / (2.0 * mf);
    }

    let intra_edges = edges
        .iter()
        .filter(|e| community[e.from_idx] == community[e.to_idx])
        .count();
    let intra_ratio = if m > 0 {
        intra_edges as f64 / m as f64
    } else {
        1.0
    };
    let gini = compute_gini(&adj);

    // ── Self node validation ──
    let mut has_self_node = false;
    let mut self_description_ok = false;
    for node in nodes {
        if node.id.contains("self") {
            has_self_node = true;
            if let MemoryType::Semantic(ref sem) = node.mem_type {
                let desc = &sem.description;
                self_description_ok = desc.contains("我");
            }
            break;
        }
    }

    // ── Failures & Warnings ──
    let mut failures: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    if avg_clustering < 0.20 {
        failures.push(format!("avg_clustering {:.3} < 0.20", avg_clustering));
    }
    if modularity < 0.06 {
        failures.push(format!("modularity {:.3} < 0.06", modularity));
    } else if modularity < 0.15 {
        warnings.push(format!(
            "modularity {:.3} < 0.15 (relaxed to 0.06 for interconnected-universe characters)",
            modularity
        ));
    }
    if intra_ratio < 0.60 {
        failures.push(format!("intra_ratio {:.2} < 0.60", intra_ratio));
    }
    if global_redundancy < 0.50 {
        failures.push(format!("global_redundancy {:.2} < 0.50", global_redundancy));
    }
    if components > 1 && min_comp_redundancy < 0.50 {
        failures.push(format!(
            "min_component_redundancy {:.2} < 0.50",
            min_comp_redundancy
        ));
    }
    if !illegal_edges.is_empty() {
        failures.push(format!("illegal_edges {}", illegal_edges.len()));
    }
    if !has_self_node {
        failures.push("missing sem_self node".to_string());
    }
    if has_self_node && !self_description_ok {
        failures.push("self description lacks 我 (first person)".to_string());
    }

    // ── Abstract situation type diversity check ──
    let mut abs_sit_types: HashSet<&'static str> = HashSet::new();
    for node in nodes {
        if let MemoryType::Situation(SituationType::AbstractSituation(ref abs)) = node.mem_type {
            let type_name = match abs {
                AbstractSituation::Location(_) => "Location",
                AbstractSituation::Participant(_) => "Participant",
                AbstractSituation::Environment(_) => "Environment",
                AbstractSituation::Event(_) => "Event",
            };
            abs_sit_types.insert(type_name);
        }
    }
    let abstract_sit_type_count = abs_sit_types.len();
    if abstract_sit_type_count < 2 {
        warnings.push(format!(
            "abstract_sit_type_count {} < 2 (need at least 2 different abstract situation types)",
            abstract_sit_type_count
        ));
    }

    // ── Procedure incoming Proc edge check ──
    let mut proc_has_incoming: HashSet<&str> = HashSet::new();
    for node in nodes {
        if classify_node_type(&node.mem_type) != "Situation" {
            continue;
        }
        for link in &node.mem_links {
            if let MemoryLinkType::Proc(_) = link.link_type {
                proc_has_incoming.insert(&link.to);
            }
        }
    }
    let mut proc_without_incoming_proc: Vec<String> = Vec::new();
    for node in nodes {
        if node.id == "proc_none" {
            continue;
        }
        if classify_node_type(&node.mem_type) == "Procedure"
            && !proc_has_incoming.contains(node.id.as_str())
        {
            proc_without_incoming_proc.push(node.id.clone());
        }
    }
    if !proc_without_incoming_proc.is_empty() {
        warnings.push(format!(
            "Proc nodes without incoming Proc edge: {}",
            proc_without_incoming_proc.join(", ")
        ));
    }

    // ── Abstract situation outgoing Proc edge check ──
    let mut abs_sit_without_proc: Vec<String> = Vec::new();
    for node in nodes {
        if let MemoryType::Situation(SituationType::AbstractSituation(_)) = node.mem_type {
            let has_proc_out = node
                .mem_links
                .iter()
                .any(|link| matches!(link.link_type, MemoryLinkType::Proc(_)));
            if !has_proc_out {
                abs_sit_without_proc.push(node.id.clone());
            }
        }
    }
    if !abs_sit_without_proc.is_empty() {
        warnings.push(format!(
            "AbstractSituation nodes without outgoing Proc edge: {}",
            abs_sit_without_proc.join(", ")
        ));
    }

    // ── proc_none existence check ──
    let has_proc_none = nodes.iter().any(|n| n.id == "proc_none");
    if !has_proc_none {
        failures.push("missing proc_none node".to_string());
    }

    GraphQualityReport {
        node_count: n,
        edge_count: m,
        node_types,
        link_types,
        connected_components: components,
        largest_component: largest,
        isolated_nodes: isolated,
        mst_forest_edges: msf,
        global_redundancy,
        avg_clustering,
        community_modularity: modularity,
        intra_community_ratio: intra_ratio,
        gini_coefficient: gini,
        has_self_node,
        self_description_ok,
        is_clean: illegal_edges.is_empty(),
        is_structurally_valid: failures.is_empty(),
        illegal_edges,
        failures,
        warnings,
        has_proc_none,
        abstract_sit_type_count,
        proc_without_incoming_proc,
        abs_sit_without_proc,
    }
}

fn compute_gini(adj: &[HashSet<usize>]) -> f64 {
    let n = adj.len();
    if n == 0 {
        return 0.0;
    }
    let mut d: Vec<f64> = adj.iter().map(|s| s.len() as f64).collect();
    if d.iter().all(|&x| x == 0.0) {
        return 0.0;
    }
    d.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let sum: f64 = d.iter().sum();
    let nf = n as f64;
    let cum: f64 = d
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64 + 1.0) * v)
        .sum();
    let g = (2.0 * cum) / (nf * sum) - (nf + 1.0) / nf;
    g.max(0.0).min(1.0)
}

pub fn print_report(r: &GraphQualityReport) {
    println!("─── Graph Quality Report ───");
    println!(
        "Nodes: {} (Sem {} / Sit {} / Proc {})",
        r.node_count, r.node_types.semantic, r.node_types.situation, r.node_types.procedure
    );
    println!(
        "Edges: {} (Sem {} / Proc {} / Sit {})",
        r.edge_count, r.link_types.sem, r.link_types.proc, r.link_types.situation
    );
    println!(
        "Components: {} | Largest: {} | Isolated: {}",
        r.connected_components, r.largest_component, r.isolated_nodes
    );
    println!(
        "Global redundancy: {:.2} (need >= 0.50)",
        r.global_redundancy
    );
    println!(
        "Clustering: {:.3} | Modularity: {:.3} | Intra: {:.2} | Gini: {:.3}",
        r.avg_clustering, r.community_modularity, r.intra_community_ratio, r.gini_coefficient
    );
    println!(
        "Self node: {} | Self desc: {}",
        if r.has_self_node { "yes" } else { "no" },
        if r.self_description_ok {
            "ok"
        } else {
            "needs 我"
        }
    );
    println!(
        "Proc none: {} | Abs sit types: {}",
        if r.has_proc_none { "yes" } else { "no" },
        r.abstract_sit_type_count
    );
    println!(
        "Illegal: {} | Structurally valid: {}",
        r.illegal_edges.len(),
        r.is_structurally_valid
    );
    for e in &r.illegal_edges {
        println!(
            "  [{}] {} --{}--> {}: {}",
            e.from_type, e.from_id, e.link_type, e.to_id, e.reason
        );
    }
    if !r.failures.is_empty() {
        println!("Failures:");
        for f in &r.failures {
            println!("  - {f}");
        }
    }
    if !r.warnings.is_empty() {
        println!("Warnings:");
        for w in &r.warnings {
            println!("  - {w}");
        }
    }
}

/// 移除所有非法边（Proc→Sit, Proc→Proc），直接修改节点
pub fn strip_illegal_edges(nodes: &mut [GraphNode]) {
    // Build type map first (immutable borrow of nodes)
    let type_map: std::collections::HashMap<String, String> = nodes
        .iter()
        .map(|n| (n.id.clone(), classify_node_type(&n.mem_type).to_string()))
        .collect();
    // Now do the mutable pass
    for node in nodes.iter_mut() {
        let from_type = classify_node_type(&node.mem_type);
        if from_type != "Procedure" {
            continue;
        }
        node.mem_links.retain(|link| {
            let to_type = type_map
                .get(&link.to)
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            to_type != "Situation" && to_type != "Procedure"
        });
    }
}

/// 确保 proc_none 节点存在，若缺失则创建一个
pub fn ensure_proc_none_node(nodes: &mut Vec<GraphNode>) {
    if nodes.iter().any(|n| n.id == "proc_none") {
        return;
    }
    let proc_none = GraphNode {
        id: "proc_none".to_string(),
        tags: vec!["无动作".to_string(), "空闲".to_string(), "默认".to_string()],
        mem_type: MemoryType::Procedure(ProcMemory::new(Action::new(
            "我暂时没有采取任何特定行动".to_string(),
            ActionType::new_think(),
        ))),
        mem_links: Vec::new(),
    };
    eprintln!("[proc_none] auto-created missing proc_none node");
    nodes.push(proc_none);
}

/// 对每个源节点的所有 Proc 出边做 softmax 归一化，使 prob 和为 1
pub fn normalize_proc_edges(nodes: &mut [GraphNode]) {
    for i in 0..nodes.len() {
        let mut proc_probs: Vec<(usize, f64)> = Vec::new();
        for (j, link) in nodes[i].mem_links.iter().enumerate() {
            if let MemoryLinkType::Proc(ref p) = link.link_type {
                proc_probs.push((j, p.prob));
            }
        }
        if proc_probs.len() < 2 {
            if proc_probs.len() == 1 && (proc_probs[0].1 - 1.0).abs() > 1e-9 {
                // Single Proc edge — set prob to 1.0
                if let MemoryLinkType::Proc(ref mut p) =
                    nodes[i].mem_links[proc_probs[0].0].link_type
                {
                    p.prob = 1.0;
                }
            }
            continue;
        }
        // Softmax normalization
        let max_p = proc_probs
            .iter()
            .map(|(_, p)| *p)
            .fold(f64::NEG_INFINITY, f64::max);
        let exps: Vec<f64> = proc_probs
            .iter()
            .map(|(_, p)| ((p - max_p) / 1.0).exp())
            .collect();
        let sum_exp: f64 = exps.iter().sum();
        if sum_exp > 0.0 {
            for (k, (idx, _)) in proc_probs.iter().enumerate() {
                if let MemoryLinkType::Proc(ref mut p) = nodes[i].mem_links[*idx].link_type {
                    p.prob = exps[k] / sum_exp;
                }
            }
        }
    }
}

/// 为每个未连接 proc_none 的 AbstractSituation 添加一条 Proc 边
/// 新边的 prob 设为该源节点现有 Proc 边 prob 的中位数（若无则用 0.5）
/// 然后对该源节点所有 Proc 边做 softmax 归一化
pub fn connect_missing_proc_none(nodes: &mut Vec<GraphNode>) {
    let has_proc_none = nodes.iter().any(|n| n.id == "proc_none");
    if !has_proc_none {
        ensure_proc_none_node(nodes);
    }

    for i in 0..nodes.len() {
        let is_abstract_sit = match nodes[i].mem_type {
            MemoryType::Situation(SituationType::AbstractSituation(_)) => true,
            _ => false,
        };
        if !is_abstract_sit {
            continue;
        }

        let source_id = nodes[i].id.clone();

        // Check if already connected to proc_none
        let already_has_none = nodes[i]
            .mem_links
            .iter()
            .any(|l| l.to == "proc_none" && matches!(l.link_type, MemoryLinkType::Proc(_)));
        if already_has_none {
            continue;
        }

        // Collect existing Proc edge probs from this source
        let mut existing_probs: Vec<f64> = nodes[i]
            .mem_links
            .iter()
            .filter_map(|l| {
                if let MemoryLinkType::Proc(ref p) = l.link_type {
                    Some(p.prob)
                } else {
                    None
                }
            })
            .collect();

        // Compute median of existing probs, default to 0.5 if empty
        let new_prob = if existing_probs.is_empty() {
            0.5
        } else {
            existing_probs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = existing_probs.len() / 2;
            if existing_probs.len() % 2 == 0 {
                (existing_probs[mid - 1] + existing_probs[mid]) / 2.0
            } else {
                existing_probs[mid]
            }
        };

        eprintln!(
            "[proc_none] connecting {} -> proc_none with initial prob {:.4}",
            source_id, new_prob
        );

        nodes[i].mem_links.push(GraphLink {
            from: source_id.clone(),
            to: "proc_none".to_string(),
            intensity: 1.0,
            link_type: MemoryLinkType::Proc(ProcMemLink { prob: new_prob }),
        });
    }

    // Re-normalize after adding missing connections
    normalize_proc_edges(nodes);
}

pub fn report_to_json(r: &GraphQualityReport) -> String {
    serde_json::to_string_pretty(r).unwrap()
}
