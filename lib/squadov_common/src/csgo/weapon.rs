use serde_repr::Serialize_repr;

#[derive(Debug, Clone, Copy, Serialize_repr)]
#[repr(i32)]
pub enum CsgoWeapon {
    Unknown,
    Knife,
    Bomb,
    Taser,
    // Pistols
    P2000,
    Usps,
    Glock,
    P250,
    FiveSeven,
    Tec9,
    Cz75,
    DuelBerettas,
    Deagle,
    R8,
    // SMG
    Mp9,
    Mac10,
    PpBizon,
    Mp7,
    Ump45,
    P90,
    Mp5,
    // Rifle
    Famas,
    Galil,
    M4a4,
    M4a1s,
    Ak47,
    Aug,
    Sg553,
    Scout,
    Awp,
    Scar20,
    G3sg1,
    // Heavy
    Nova,
    Xm1014,
    Mag7,
    SawedOff,
    M249,
    Negev,
    // Grenades
    He,
    Flashbang,
    Smoke,
    Decoy,
    Molotov,
    Incendiary,
}

pub fn csgo_string_to_weapon(class: &str) -> CsgoWeapon {
    let test_class = class.to_lowercase();

    if test_class.starts_with("cweapon") {
        csgo_string_to_weapon(test_class.strip_prefix("cweapon").unwrap())
    } else if test_class.contains("knife") {
        CsgoWeapon::Knife
    } else if test_class.starts_with("weapon_") {
        csgo_string_to_weapon(test_class.strip_prefix("weapon_").unwrap())
    } else {
        match test_class.as_str() {
            // Grenades
            "csmokegrenade" | "smoke" | "smokegrenade" => CsgoWeapon::Smoke,
            "cmolotovgrenade" | "molotov" | "molotovgrenade" | "molotov_projectile" => CsgoWeapon::Molotov,
            "cincendiarygrenade" | "incendiary" | "incgrenade" | "incendiarygrenade" | "inferno" => CsgoWeapon::Incendiary,
            "chegrenade" | "he" | "hegrenade" => CsgoWeapon::He,
            "cdecoygrenade" | "decoy" | "decoygrenade" => CsgoWeapon::Decoy,
            "cflashbang" | "flashbang" => CsgoWeapon::Flashbang,
            // Other
            "cc4" => CsgoWeapon::Bomb,
            "taser" => CsgoWeapon::Taser,
            // Pistols
            "hkp2000" => CsgoWeapon::P2000,
            "usp" | "usp_silencer" | "usp_silencer_off" => CsgoWeapon::Usps,
            "glock" => CsgoWeapon::Glock,
            "p250" => CsgoWeapon::P250,
            "fiveseven" => CsgoWeapon::FiveSeven,
            "tec9" => CsgoWeapon::Tec9,
            "cz75a" => CsgoWeapon::Cz75,
            "elite" => CsgoWeapon::DuelBerettas,
            "cdeagle" | "deagle" => CsgoWeapon::Deagle,
            "revolver" => CsgoWeapon::R8,
            // SMG
            "mp9" => CsgoWeapon::Mp9,
            "mac10" => CsgoWeapon::Mac10,
            "bizon" => CsgoWeapon::PpBizon,
            "mp7" => CsgoWeapon::Mp7,
            "ump45" => CsgoWeapon::Ump45,
            "p90" => CsgoWeapon::P90,
            "mp5sd" | "mp5navy" => CsgoWeapon::Mp5,
            // Rifle
            "famas" => CsgoWeapon::Famas,
            "galilar" | "galil" => CsgoWeapon::Galil,
            "m4a1" => CsgoWeapon::M4a4,
            "m4a1_silencer" | "m4a1_silencer_off" => CsgoWeapon::M4a1s,
            "ak47" | "cak47" => CsgoWeapon::Ak47,
            "aug" => CsgoWeapon::Aug,
            "sg556" => CsgoWeapon::Sg553,
            "ssg08" | "scout" => CsgoWeapon::Scout,
            "awp" => CsgoWeapon::Awp,
            "scar20" => CsgoWeapon::Scar20,
            "g3sg1" => CsgoWeapon::G3sg1,
            // Heavy
            "nova" => CsgoWeapon::Nova,
            "xm1014" => CsgoWeapon::Xm1014,
            "mag7" => CsgoWeapon::Mag7,
            "sawedoff" => CsgoWeapon::SawedOff,
            "m249" => CsgoWeapon::M249,
            "negev" => CsgoWeapon::Negev,
            _ => {
                log::info!("Unknown CSGO weapon string: {}", &test_class);
                CsgoWeapon::Unknown
            },
        }
    }
}