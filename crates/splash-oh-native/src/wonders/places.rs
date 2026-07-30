//! Where each wonder is, what film the editorial links to, and the twenty-five
//! collectibles hidden through the app.
//!
//! `videoId` and `lat`/`lng` come from each wonder's `*_data.dart`; the
//! collectibles from `collectible_data.dart`, in its order.

/// (YouTube id, latitude, longitude) per wonder, in the order `WONDERS` lists
/// them.
pub const PLACES: &[(&str, f64, f64)] = &[
    ("lJKX3Y7Vqvs", 29.9792, 31.1342),
    ("do1Go22Wu8o", 40.43199751120627, 116.57040708482984),
    ("ezDiSkOU0wc", 30.328830750209903, 35.44398203484667),
    ("GXoEpNjgKzg", 41.890242126393495, 12.492349361871392),
    ("Q6eBJjdca14", 20.68346184201756, -88.56769676930931),
    ("cnMa-Sm9H4k", -13.162690683637758, -72.54500778824891),
    ("EWkDzLrhpXI", 27.17405039840427, 78.04211890065208),
    ("k_615AauSds", -22.95238891944396, -43.21045520611561),
];

/// One collectible: what it is called, which wonder hides it, the Met object it
/// unlocks, and which of the five icons stands for it.
pub struct Collectible {
    pub title: &'static str,
    pub wonder: usize,
    pub artifact_id: &'static str,
    pub icon: &'static str,
}

pub const COLLECTIBLES: &[Collectible] = &[
    Collectible {
        title: "Pendant",
        wonder: 4,
        artifact_id: "701645",
        icon: "jewelry",
    },
    Collectible {
        title: "Bird Ornament",
        wonder: 4,
        artifact_id: "310555",
        icon: "jewelry",
    },
    Collectible {
        title: "La Prison, à Chichen-Itza",
        wonder: 4,
        artifact_id: "286467",
        icon: "picture",
    },
    Collectible {
        title: "Engraved Horn",
        wonder: 7,
        artifact_id: "501302",
        icon: "statue",
    },
    Collectible {
        title: "Fixed fan",
        wonder: 7,
        artifact_id: "157985",
        icon: "jewelry",
    },
    Collectible {
        title: "Handkerchiefs (one of two)",
        wonder: 7,
        artifact_id: "227759",
        icon: "textile",
    },
    Collectible {
        title: "Glass hexagonal amphoriskos",
        wonder: 3,
        artifact_id: "245376",
        icon: "vase",
    },
    Collectible {
        title: "Bronze plaque of Mithras slaying the bull",
        wonder: 3,
        artifact_id: "256570",
        icon: "statue",
    },
    Collectible {
        title: "Interno del Colosseo",
        wonder: 3,
        artifact_id: "286136",
        icon: "picture",
    },
    Collectible {
        title: "Biographies of Lian Po and Lin Xiangru",
        wonder: 1,
        artifact_id: "39918",
        icon: "scroll",
    },
    Collectible {
        title: "Jar with Dragon",
        wonder: 1,
        artifact_id: "39666",
        icon: "vase",
    },
    Collectible {
        title: "Panel with Peonies and Butterfly",
        wonder: 1,
        artifact_id: "39735",
        icon: "textile",
    },
    Collectible {
        title: "Eight-Pointed Star Tunic",
        wonder: 5,
        artifact_id: "308120",
        icon: "textile",
    },
    Collectible {
        title: "Camelid figurine",
        wonder: 5,
        artifact_id: "309960",
        icon: "statue",
    },
    Collectible {
        title: "Double Bowl",
        wonder: 5,
        artifact_id: "313341",
        icon: "vase",
    },
    Collectible {
        title: "Camel and riders",
        wonder: 2,
        artifact_id: "322592",
        icon: "statue",
    },
    Collectible {
        title: "Vessel",
        wonder: 2,
        artifact_id: "325918",
        icon: "vase",
    },
    Collectible {
        title: "Open bowl",
        wonder: 2,
        artifact_id: "326243",
        icon: "vase",
    },
    Collectible {
        title: "Two papyrus fragments",
        wonder: 0,
        artifact_id: "546510",
        icon: "scroll",
    },
    Collectible {
        title: "Fragmentary Face of King Khafre",
        wonder: 0,
        artifact_id: "543896",
        icon: "statue",
    },
    Collectible {
        title: "Jewelry Elements",
        wonder: 0,
        artifact_id: "545728",
        icon: "jewelry",
    },
    Collectible {
        title: "Dagger with Scabbard",
        wonder: 6,
        artifact_id: "24907",
        icon: "jewelry",
    },
    Collectible {
        title: "The House of Bijapur",
        wonder: 6,
        artifact_id: "453183",
        icon: "picture",
    },
    Collectible {
        title: "Panel of Nasta'liq Calligraphy",
        wonder: 6,
        artifact_id: "453983",
        icon: "scroll",
    },
];
