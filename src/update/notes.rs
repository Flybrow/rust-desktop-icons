const LEAD: usize = 110;

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Section(String),
    Item(String, String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Notes {
    pub title: String,
    pub blocks: Vec<Block>,
}

fn sentence(text: &str) -> String {
    let end = [". ", "; "].iter().filter_map(|p| text.find(p)).min().unwrap_or(text.len());
    let first = text[..end].trim().trim_end_matches([';', '.']);
    if first.chars().count() <= LEAD {
        return first.to_string();
    }
    let cut: String = first.chars().take(LEAD).collect();
    format!("{}…", cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head))
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}

fn item(text: &str) -> Block {
    let text = text.replace('`', "");
    match text.strip_prefix("**").and_then(|r| r.split_once("**")) {
        Some((bold, rest)) => {
            let rest = rest.trim().trim_start_matches([':', ',']).trim().replace("**", "");
            Block::Item(bold.trim().trim_end_matches(':').trim().to_string(), capital(&sentence(&rest)))
        }
        None => Block::Item(sentence(&text.replace("**", "")), String::new()),
    }
}

pub fn parse(md: &str) -> Notes {
    let mut notes = Notes::default();
    for l in md.lines().map(str::trim) {
        if let Some(h) = l.strip_prefix("### ") {
            notes.blocks.push(Block::Section(h.trim().to_uppercase()));
        } else if let Some(t) = l.strip_prefix("## ") {
            notes.title = t.trim().to_string();
        } else if let Some(b) = l.strip_prefix("- ") {
            notes.blocks.push(item(b));
        }
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_become_sections_and_items() {
        let md = "## What's new\n\nIntro.\n\n### New features\n- **Peek:** press `Ctrl`. More\n- Plain item; more\n\n### Fixes\n- **Sort**, by size\n- **Rules:** match *.pdf files. x";
        let n = parse(md);
        assert_eq!(n.title, "What's new");
        assert_eq!(
            n.blocks,
            vec![
                Block::Section("NEW FEATURES".into()),
                Block::Item("Peek".into(), "Press Ctrl".into()),
                Block::Item("Plain item".into(), String::new()),
                Block::Section("FIXES".into()),
                Block::Item("Sort".into(), "By size".into()),
                Block::Item("Rules".into(), "Match *.pdf files".into()),
            ]
        );
        let long = format!("- **T:** {} end.", "word ".repeat(40));
        let Block::Item(_, d) = &parse(&long).blocks[0] else { panic!() };
        assert!(d.ends_with("word…") && d.chars().count() <= LEAD + 1);
        assert_eq!(parse("").title, "");
    }
}
