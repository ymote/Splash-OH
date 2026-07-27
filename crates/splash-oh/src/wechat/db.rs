//! The data behind the WeChat demo, ported from `src/api.rs` in
//! [project-robius/makepad_wechat](https://github.com/project-robius/makepad_wechat).
//!
//! Same twelve chats with the same names, the same CJK message bodies, and the
//! same `chat_id` filtering scheme. Both the Rust and the ArkTS implementation
//! read from this — the ArkTS side gets it over napi — so neither can win by
//! rendering less content than the other.

/// What the chat list shows under a name.
#[derive(Clone, Copy)]
pub enum Preview {
    Audio,
    Image,
    Video,
    Text(&'static str),
}

impl Preview {
    pub fn text(&self) -> &'static str {
        match self {
            Preview::Audio => "[Audio]",
            Preview::Image => "[Image]",
            Preview::Video => "[Video]",
            Preview::Text(t) => t,
        }
    }
}

pub struct Chat {
    pub id: u64,
    pub username: &'static str,
    pub preview: Preview,
    pub timestamp: &'static str,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    Incoming,
    Outgoing,
}

pub struct Message {
    pub direction: Direction,
    pub chat_id: u64,
    pub text: &'static str,
}

/// The twelve chats, in the reference app's order.
pub const CHATS: &[Chat] = &[
    Chat { id: 1,  username: "Olive Yew",        preview: Preview::Text("Hey, how are you?"),        timestamp: "10:45" },
    Chat { id: 2,  username: "John Doe",         preview: Preview::Audio,                            timestamp: "09:12" },
    Chat { id: 3,  username: "Peg Legge",        preview: Preview::Text("See you tomorrow!"),        timestamp: "Yesterday" },
    Chat { id: 4,  username: "Barb Akew",        preview: Preview::Image,                            timestamp: "Yesterday" },
    Chat { id: 5,  username: "Chris P. Bacon",   preview: Preview::Text("Sounds good to me"),        timestamp: "Yesterday" },
    Chat { id: 6,  username: "WeChat Team",      preview: Preview::Text("Welcome to WeChat"),        timestamp: "Monday" },
    Chat { id: 7,  username: "Andrew Lin",       preview: Preview::Video,                            timestamp: "Monday" },
    Chat { id: 8,  username: "Christian Huxley", preview: Preview::Text("Did you see the build?"),   timestamp: "Sunday" },
    Chat { id: 9,  username: "Ana Leddie",       preview: Preview::Text("Thanks!"),                  timestamp: "Sunday" },
    Chat { id: 10, username: "Adam Adler",       preview: Preview::Text("Let's meet at 3pm"),        timestamp: "12/04" },
    Chat { id: 11, username: "Gabriel Hayes",    preview: Preview::Audio,                            timestamp: "12/03" },
    Chat { id: 12, username: "Eric Ford",        preview: Preview::Text("I'm using WeChat"),         timestamp: "12/01" },
];

/// The message bodies, verbatim from the reference app.
const BODIES: &[(Direction, &str)] = &[
    (Direction::Incoming, "体議速人幅触無持編聞組込"),
    (Direction::Outgoing, "減活乗治外進"),
    (Direction::Incoming, "福読併棋一御質慰"),
    (Direction::Outgoing, "嶋可済政実玉全強無示餌"),
    (Direction::Outgoing, "福読併棋一御質慰"),
    (Direction::Incoming, "消再野誰強心無嶋可済実玉全示餌"),
    (Direction::Outgoing, "体議速人幅触無持編聞組込"),
    (Direction::Incoming, "減活乗治外進"),
];

/// How many messages a chat has. The reference app builds 200 × 8 entries and
/// filters by `chat_id`, which works out to roughly this many per chat.
pub const MESSAGES_PER_CHAT: usize = 32;

/// Message `i` of `chat_id`.
pub fn message(chat_id: u64, i: usize) -> Message {
    let (direction, text) = BODIES[(i + chat_id as usize) % BODIES.len()];
    Message {
        direction,
        chat_id,
        text,
    }
}

pub fn chat(id: u64) -> Option<&'static Chat> {
    CHATS.iter().find(|c| c.id == id)
}

/// Contacts, grouped by initial, as the reference app's contacts list is.
pub const CONTACT_GROUPS: &[(&str, &[&str])] = &[
    ("A", &["Adam Adler", "Ana Leddie", "Andrew Lin", "Aaron Pike"]),
    ("B", &["Barb Akew", "Ben Dover", "Bill Ding"]),
    ("C", &["Chris P. Bacon", "Christian Huxley", "Carrie Oki"]),
    ("E", &["Eric Ford", "Ella Vator", "Earl E. Bird"]),
    ("G", &["Gabriel Hayes", "Gail Forcewind"]),
    ("J", &["John Doe", "Jorge Bejar", "Julian Montes de Oca", "Jo King"]),
    ("O", &["Olive Yew", "Olive Branch"]),
    ("P", &["Peg Legge", "Paige Turner", "Polly Ester"]),
    ("R", &["Rik Arends", "Rita Book", "Robin Banks"]),
    ("W", &["Warren Peace", "Wilma Mine"]),
];

/// The fixed entries above the contacts list.
pub const CONTACT_ACTIONS: &[&str] = &[
    "New Friends",
    "Group Chats",
    "Tags",
    "Official Accounts",
    "WeCom Contacts",
];

/// Discover screen entries, in the reference app's grouping.
pub const DISCOVER_GROUPS: &[&[&str]] = &[
    &["Moments"],
    &["Channels", "Live"],
    &["Scan", "Shake"],
    &["Top Stories", "Search"],
    &["Mini Programs"],
];

/// Profile screen entries.
pub const PROFILE_GROUPS: &[&[&str]] = &[
    &["Services"],
    &["Favorites", "Moments", "Cards & Offers", "Sticker Gallery"],
    &["Settings"],
];

/// Moments feed: (author, body, whether it carries a photo).
pub const MOMENTS: &[(&str, &str, bool)] = &[
    ("Olive Yew", "体議速人幅触無持編聞組込", true),
    ("John Doe", "減活乗治外進", false),
    ("Peg Legge", "福読併棋一御質慰", true),
    ("Barb Akew", "嶋可済政実玉全強無示餌", false),
    ("Chris P. Bacon", "消再野誰強心無嶋可済実玉全示餌", true),
    ("WeChat Team", "Welcome to WeChat Moments", false),
    ("Andrew Lin", "体議速人幅触無持編聞組込", true),
    ("Ana Leddie", "減活乗治外進", false),
];
