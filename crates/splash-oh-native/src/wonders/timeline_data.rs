//! Dates and events, from each wonder's `*_data.dart`.
//!
//! `start_yr`/`end_yr` are the construction span the timeline plots; the events
//! are the entries the app lists per wonder -- six each -- with their years. Negative
//! years are BCE.

pub struct Event {
    pub year: i32,
    pub text: &'static str,
}

pub struct Timeline {
    pub start_yr: i32,
    pub end_yr: i32,
    pub events: &'static [Event],
}

const PYRAMIDS_EVENTS: &[Event] = &[
    Event { year: -2575, text: "Construction of the 3 pyramids began for three kings of the 4th dynasty; Khufu, Khafre, and Menkaure." },
    Event { year: -2465, text: "Construction began on the smaller surrounding structures called Mastabas for royalty of the 5th and 6th dynasties." },
    Event { year: -443, text: "Greek Author Herodotus speculated that the pyramids were built in the span of 20 years with over 100,000 slave labourers. This assumption would last for over 1500 years" },
    Event { year: 1925, text: "Tomb of Queen Hetepheres was discovered, containing furniture and jewelry. One of the last remaining treasure-filled tombs after many years of looting and plundering." },
    Event { year: 1979, text: "Designated a UNESCO World Heritage Site to prevent any more unauthorized plundering and vandalism." },
    Event { year: 1990, text: "Discovery of labouror’s districts suggest that the workers building the pyramids were not slaves, and an ingenious building method proved a relatively small work-force was required to build such immense structures." },
];

const GREAT_WALL_OF_CHINA_EVENTS: &[Event] = &[
    Event { year: -700, text: "First landmark of the Great Wall began originally as a square wall surrounding the state of Chu. Over the years, additional walls would be built and added to it to expand and connect territory." },
    Event { year: -214, text: "The first Qin Emperor unifies China and links the wall of the surrounding states of Qin, Yan, and Zhao into the Great Wall of China, taking 10 years to build with hundreds of thousands of laborers." },
    Event { year: -121, text: "A 20-year construction project was started by the Han emperor to build east and west sections of the wall, including beacons, towers, and castles. Not just for defense, but also to control trade routes like the Silk Road." },
    Event { year: 556, text: "The Bei Qi kingdom also launched several construction projects, utilizing over 1.8 million workers to repair and extend sections of the wall, adding to its length and even building a second inner wall around Shanxi." },
    Event { year: 618, text: "The Great Wall was repaired during the Sui Dynasty and used to defend against Tujue attacks. Before and after the Sui Dynasty, the wall saw very little use and fell into disrepair." },
    Event { year: 1487, text: "Hongzhi Emperor split the walls into north and south lines, eventually shaping it into how it is today. Since then, it has gradually fallen into disrepair and remains mostly unused." },
];

const PETRA_EVENTS: &[Event] = &[
    Event { year: -1200, text: "First Edomites occupied the area and established a foothold." },
    Event { year: -106, text: "Became part of the Roman province Arabia" },
    Event { year: 551, text: "After being damaged by earthquakes, habitation of the city all but ceased." },
    Event { year: 1812, text: "Rediscovered by the Swiss traveler Johann Ludwig Burckhardt." },
    Event { year: 1958, text: "Excavations led on the site by the British School of Archaeology and the American Center of Oriental Research." },
    Event { year: 1989, text: "Appeared in the film Indiana Jones and The Last Crusade." },
];

const COLOSSEUM_EVENTS: &[Event] = &[
    Event { year: 70, text: "Colosseum construction was started during the Vespasian reign overtop what used to be a private lake for the previous four emperors. This was done in an attempt to revitalize Rome from their tyrannical reign." },
    Event { year: 82, text: "The uppermost floor was built, and the structure was officially completed by Domitian." },
    Event { year: 1140, text: "The arena was repurposed as a fortress for the Frangipane and Annibaldi families. It was also at one point used as a church." },
    Event { year: 1490, text: "Pope Alexander VI permitted the site to be used as a quarry, for both storing and salvaging building materials." },
    Event { year: 1829, text: "Preservation of the colosseum officially began, after more than a millennia of dilapidation and vandalism. Pope Pius VIII was notably devoted to this project." },
    Event { year: 1990, text: "A restoration project was undertaken to ensure the colosseum remained a major tourist attraction for Rome. It currently stands as one of the greatest sources of tourism revenue in Italy." },
];

const CHICHEN_ITZA_EVENTS: &[Event] = &[
    Event {
        year: 600,
        text:
            "Chichen Itza rises to regional prominence toward the end of the Early Classic period",
    },
    Event {
        year: 832,
        text: "The earliest hieroglyphic date discovered at Chichen Itza",
    },
    Event {
        year: 998,
        text: "Last known date recorded in the Osario temple",
    },
    Event {
        year: 1100,
        text: "Chichen Itza declines as a regional center",
    },
    Event {
        year: 1527,
        text: "Invaded by Spanish Conquistador Francisco de Montejo",
    },
    Event {
        year: 1535,
        text: "All Spanish are driven from the Yucatán Peninsula",
    },
];

const MACHU_PICCHU_EVENTS: &[Event] = &[
    Event { year: 1438, text: "Speculated to be built and occupied by Inca ruler Pachacuti Inca Yupanqui." },
    Event { year: 1572, text: "The last Inca rulers used the site as a bastion to rebel against Spanish rule until they were ultimately wiped out." },
    Event { year: 1867, text: "Speculated to have been originally discovered by German explorer Augusto Berns, but his findings were never effectively publicized." },
    Event { year: 1911, text: "Introduced to the world by Hiram Bingham of Yale University, who was led there by locals after disclosing he was searching for Vilcabamba, the ’lost city of the Incas’." },
    Event { year: 1964, text: "Surrounding sites were excavated thoroughly by Gene Savoy, who found a much more suitable candidate for Vilcabamba in the ruin known as Espíritu Pampa." },
    Event { year: 1997, text: "Since its rediscovery, growing numbers of tourists have visited the Machu Picchu each year, with numbers exceeding 1.4 million in 2017." },
];

const TAJ_MAHAL_EVENTS: &[Event] = &[
    Event { year: 1631, text: "Built by Mughal Emperor Shah Jahān to immortalize his deceased wife." },
    Event { year: 1647, text: "Construction completed. The project involved over 20,000 workers and spanned 42 acres." },
    Event { year: 1658, text: "There were plans for a second mausoleum for his own remains, but Shah Jahān was imprisoned by his son for the rest of his life in Agra Fort, and this never came to pass." },
    Event { year: 1901, text: "Lord Curzon and the British Viceroy of India carried out a major restoration to the monument after over 350 years of decay and corrosion due to factory pollution and exhaust." },
    Event { year: 1984, text: "To protect the structure from Sikh militants and some Hindu nationalist groups, night viewing was banned to tourists. This ban would last 20 years." },
    Event { year: 1998, text: "Restoration and research program put into action to help preserve the monument." },
];

const CHRIST_THE_REDEEMER_EVENTS: &[Event] = &[
    Event { year: 1850, text: "Plans for the statue were first proposed by Pedro Maria Boss upon Mount Corcovado. This was never approved, however." },
    Event { year: 1921, text: "A new plan was proposed by the Roman Catholic archdiocese, and after the citizens of Rio de Janeiro petitioned the president, it was finally approved." },
    Event { year: 1922, text: "The foundation of the statue was ceremoniously laid out to commemorate Brazil’s independence from Portugal." },
    Event { year: 1926, text: "Construction officially began after the initial design was chosen via a competition and amended by Brazilian artists and engineers." },
    Event { year: 1931, text: "Construction of the statue was completed, standing 98’ tall with a 92’ wide arm span." },
    Event { year: 2006, text: "A chapel was consecrated at the statue’s base to Our Lady of Aparecida to mark the statue’s 75th anniversary." },
];

/// Same order as `WONDERS`.
pub const TIMELINES: &[Timeline] = &[
    Timeline {
        start_yr: -2600,
        end_yr: -2500,
        events: PYRAMIDS_EVENTS,
    },
    Timeline {
        start_yr: -700,
        end_yr: 1644,
        events: GREAT_WALL_OF_CHINA_EVENTS,
    },
    Timeline {
        start_yr: -312,
        end_yr: 100,
        events: PETRA_EVENTS,
    },
    Timeline {
        start_yr: 70,
        end_yr: 80,
        events: COLOSSEUM_EVENTS,
    },
    Timeline {
        start_yr: 550,
        end_yr: 1550,
        events: CHICHEN_ITZA_EVENTS,
    },
    Timeline {
        start_yr: 1450,
        end_yr: 1572,
        events: MACHU_PICCHU_EVENTS,
    },
    Timeline {
        start_yr: 1632,
        end_yr: 1653,
        events: TAJ_MAHAL_EVENTS,
    },
    Timeline {
        start_yr: 1922,
        end_yr: 1931,
        events: CHRIST_THE_REDEEMER_EVENTS,
    },
];
