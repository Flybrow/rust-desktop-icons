use super::{DESCRIBED, HEADER, ROW, Ui, f, prompt};
use serde_json::json;

pub fn rules(u: &mut Ui) {
    let n_tabs = u.cfg()["fences"].as_array().map_or(0, |a| a.len());
    let mut rules = vec![HEADER, DESCRIBED];
    rules.extend(std::iter::repeat_n(DESCRIBED, n_tabs));
    rules.extend([DESCRIBED, HEADER, DESCRIBED, HEADER, DESCRIBED]);
    let (add_name, add_age) = (n_tabs + 4, n_tabs + 6);
    u.page(5);
    u.row(&rules, 1);
    u.check(u.until(|c| c["auto_sort"] == true), "Sorting rules → automatic sorting toggles");
    u.row(&rules, 1);
    u.until(|c| c["auto_sort"] == false);
    u.row(&rules, 2);
    u.pick(&["Images"]);
    u.check(u.until(|c| f(c, 0, "tabs")[0]["kinds"] == json!(["Images"])), "Sorting rules → a fence row picks the file types it takes");
    u.row(&rules, add_name);
    prompt("*.e2e");
    u.pick(&["Archive"]);
    u.check(u.until(|c| c["name_rules"] == json!([{"pattern": "*.e2e", "fence": 2, "tab": 2}])), "Add a rule by name: pattern, then fence");
    let mut rules_named = rules.clone();
    rules_named.insert(add_name, ROW);
    u.row(&rules_named, add_name);
    u.pick(&["Edit the name pattern"]);
    prompt("*.log");
    u.check(u.until(|c| c["name_rules"][0]["pattern"] == "*.log"), "a name rule's pattern can be edited");
    u.row(&rules_named, add_name);
    u.pick(&["Change the destination fence", "Docs"]);
    u.check(u.until(|c| c["name_rules"][0]["fence"] == 1), "a name rule's fence can be changed");
    u.row(&rules_named, add_name);
    u.pick(&["Delete rule"]);
    u.check(u.until(|c| c["name_rules"].as_array().is_none_or(|a| a.is_empty())), "a name rule can be deleted");
    u.row(&rules, add_age);
    u.pick(&["Docs"]);
    u.pick(&["Older than 60 days"]);
    u.pick(&["Archive"]);
    let age = json!({"days": 60, "fence": 2, "tab": 2});
    u.check(u.until(|c| f(c, 0, "tabs")[0]["expire"] == age), "Add a rule by age: fence, age, destination");
    let mut rules_aged = rules.clone();
    rules_aged.insert(add_age, ROW);
    u.row(&rules_aged, add_age);
    u.pick(&["Older than 60 days"]);
    u.pick(&["Older than 90 days"]);
    u.check(u.until(|c| f(c, 0, "tabs")[0]["expire"]["days"] == 90), "an age rule's age can be changed");
    u.row(&rules_aged, add_age);
    u.pick(&["Delete rule"]);
    u.check(u.until(|c| f(c, 0, "tabs")[0]["expire"].is_null()), "an age rule can be deleted");
}

pub fn privacy(u: &mut Ui) {
    let private = [HEADER, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED, DESCRIBED];
    u.page(7);
    u.row(&private, 1);
    let on = u.until(|c| c["privacy"]["on"] == true);
    u.row(&private, 4);
    let titles = u.until(|c| c["privacy"]["titles"] == true);
    u.row(&private, 5);
    u.pick(&["Never"]);
    let never = u.until(|c| c["privacy"]["reveal"] == "Never");
    u.row(&private, 1);
    u.check(on && titles && never && u.until(|c| c["privacy"]["on"] == false), "Privacy → mode, titles and reveal are saved");
}
