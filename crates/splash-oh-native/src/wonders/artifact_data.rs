//! Artifacts, from the Metropolitan Museum collection.
//!
//! Wonderous lists four highlight artifacts per wonder by Met object id and
//! fetches each one at runtime. The ids are the app's; the titles, dates and
//! cultures come from the Met's public collection API, and the photographs from
//! the same host the app uses. Both were fetched once and are shipped, because
//! an app that needs the network to show its own content is a different app.

pub struct Artifact {
    pub id: &'static str,
    pub title: &'static str,
    pub date: &'static str,
    pub culture: &'static str,
}

const PYRAMIDS_ARTIFACTS: &[Artifact] = &[
    Artifact {
        id: "543864",
        title: "Guardian Figure",
        date: "ca. 1919–1885 B.C.",
        culture: "Egyptian Art",
    },
    Artifact {
        id: "546488",
        title: "Relief fragment",
        date: "ca. 1981–1640 B.C.",
        culture: "Egyptian Art",
    },
    Artifact {
        id: "557137",
        title: "Ring with Uninscribed Scarab",
        date: "ca. 1850–1640 B.C.",
        culture: "Egyptian Art",
    },
    Artifact {
        id: "543900",
        title: "Nikare as a scribe",
        date: "ca. 2420–2389 B.C. or later",
        culture: "Egyptian Art",
    },
];

const GREAT_WALL_OF_CHINA_ARTIFACTS: &[Artifact] = &[
    Artifact {
        id: "79091",
        title: "Cape",
        date: "second half 16th century",
        culture: "French",
    },
    Artifact {
        id: "781812",
        title: "Censer in the form of a mythical beast",
        date: "early 17th century",
        culture: "China",
    },
    Artifact {
        id: "40213",
        title: "Dish with peafowls and peonies",
        date: "early 15th century",
        culture: "China",
    },
    Artifact {
        id: "40765",
        title: "Base for a mandala",
        date: "first half 15th century",
        culture: "China",
    },
];

const PETRA_ARTIFACTS: &[Artifact] = &[
    Artifact {
        id: "325900",
        title: "Unguentarium",
        date: "ca. 1st century CE",
        culture: "Nabataean",
    },
    Artifact {
        id: "325902",
        title: "Cooking pot",
        date: "ca. 1st century CE",
        culture: "Nabataean",
    },
    Artifact {
        id: "325919",
        title: "Lamp",
        date: "ca. 1st century CE",
        culture: "Nabataean",
    },
    Artifact {
        id: "325884",
        title: "Bowl",
        date: "ca. 1st century CE",
        culture: "Nabataean",
    },
];

const COLOSSEUM_ARTIFACTS: &[Artifact] = &[
    Artifact {
        id: "251350",
        title: "Marble portrait of a young woman",
        date: "150–175 CE",
        culture: "Roman",
    },
    Artifact {
        id: "255960",
        title: "Silver mirror",
        date: "4th century CE",
        culture: "Roman",
    },
    Artifact {
        id: "247993",
        title: "Marble portrait of the emperor Augustus",
        date: "ca. 14–37 CE",
        culture: "Roman",
    },
    Artifact {
        id: "250464",
        title: "Terracotta medallion",
        date: "late 2nd–early 3rd century CE",
        culture: "Roman",
    },
];

const CHICHEN_ITZA_ARTIFACTS: &[Artifact] = &[
    Artifact {
        id: "503940",
        title: "Double Whistle",
        date: "7th–9th century",
        culture: "Mayan",
    },
    Artifact {
        id: "312595",
        title: "Seated female figure",
        date: "700–800 CE",
        culture: "Maya",
    },
    Artifact {
        id: "310551",
        title: "Censer Support",
        date: "mid-7th–9th century",
        culture: "Maya",
    },
    Artifact {
        id: "316304",
        title: "Tripod Plate",
        date: "9th–10th century",
        culture: "Maya",
    },
];

const MACHU_PICCHU_ARTIFACTS: &[Artifact] = &[
    Artifact {
        id: "313295",
        title: "Beaker with face",
        date: "1400–1535 CE",
        culture: "Inca",
    },
    Artifact {
        id: "316926",
        title: "Feathered Bag",
        date: "15th–early 16th century",
        culture: "Inca",
    },
    Artifact {
        id: "309944",
        title: "Miniature female effigy",
        date: "1400–1535 CE",
        culture: "Inca",
    },
    Artifact {
        id: "309436",
        title: "Stirrup Spout Bottle with Felines",
        date: "4th–7th century",
        culture: "Moche",
    },
];

const TAJ_MAHAL_ARTIFACTS: &[Artifact] = &[
    Artifact {
        id: "453341",
        title: "Mango-Shaped Flask",
        date: "mid-17th century",
        culture: "Islamic Art",
    },
    Artifact {
        id: "453243",
        title: "Base for a Water Pipe (Huqqa) with Irises",
        date: "late 17th century",
        culture: "Islamic Art",
    },
    Artifact {
        id: "73309",
        title: "Plate",
        date: "mid-16th–17th century",
        culture: "India (Gujarat)",
    },
    Artifact {
        id: "24932",
        title: "Helmet",
        date: "18th century",
        culture: "Indian, Mughal",
    },
];

const CHRIST_THE_REDEEMER_ARTIFACTS: &[Artifact] = &[
    Artifact {
        id: "501319",
        title: "Pluriarc",
        date: "late 19th century",
        culture: "African American (Brazil - Afro-Brazilian?)",
    },
    Artifact {
        id: "764815",
        title: "[Studio Portrait: Male Street Vendor Holding Box of Flowers, Brazil]",
        date: "1864–66",
        culture: "Photographs",
    },
    Artifact {
        id: "502019",
        title: "Strung Rattle",
        date: "19th century",
        culture: "Native American (Brazilian)",
    },
    Artifact {
        id: "764814",
        title: "[Studio Portrait: Two Males Wearing Hats and Ponchos, Brazil]",
        date: "1864–66",
        culture: "Photographs",
    },
];

/// Same order as `WONDERS`.
pub const ARTIFACTS: &[&[Artifact]] = &[
    PYRAMIDS_ARTIFACTS,
    GREAT_WALL_OF_CHINA_ARTIFACTS,
    PETRA_ARTIFACTS,
    COLOSSEUM_ARTIFACTS,
    CHICHEN_ITZA_ARTIFACTS,
    MACHU_PICCHU_ARTIFACTS,
    TAJ_MAHAL_ARTIFACTS,
    CHRIST_THE_REDEEMER_ARTIFACTS,
];
