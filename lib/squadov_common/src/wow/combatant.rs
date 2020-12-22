use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct WoWCombatantInfo {
    pub guid: String
}

pub fn generate_combatants_key(combatants: &[WoWCombatantInfo]) -> String {
    let mut sorted_combatants = combatants.to_vec();
    sorted_combatants.sort_by(|a, b| {
        if a.guid < b.guid {
            return std::cmp::Ordering::Less;
        } else if a.guid > b.guid {
            return std::cmp::Ordering::Greater;
        } else {
            return std::cmp::Ordering::Equal;
        }
    });

    let combined_guids: Vec<String> = sorted_combatants.iter().map(|x| { x.guid.clone() }).collect();
    combined_guids.join("::")
}