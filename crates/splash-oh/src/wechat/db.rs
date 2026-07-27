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
    /// Avatar file in `rawfile/wechat/`, from the reference app's own set.
    pub avatar: &'static str,
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
    Chat { id: 1, username: "Olive Yew", preview: Preview::Text("Hey, how are you?"), timestamp: "10:45", avatar: "user1.png" },
    Chat { id: 2, username: "John Doe", preview: Preview::Audio, timestamp: "09:12", avatar: "user2.png" },
    Chat { id: 3, username: "Peg Legge", preview: Preview::Text("See you tomorrow!"), timestamp: "Yesterday", avatar: "user3.png" },
    Chat { id: 4, username: "Barb Akew", preview: Preview::Image, timestamp: "Yesterday", avatar: "user4.png" },
    Chat { id: 5, username: "Chris P. Bacon", preview: Preview::Text("Sounds good to me"), timestamp: "Yesterday", avatar: "user5.png" },
    Chat { id: 6, username: "WeChat Team", preview: Preview::Text("Welcome to WeChat"), timestamp: "Monday", avatar: "wechat_avatar.png" },
    Chat { id: 7, username: "Andrew Lin", preview: Preview::Video, timestamp: "Monday", avatar: "user6.png" },
    Chat { id: 8, username: "Christian Huxley", preview: Preview::Text("Did you see the build?"), timestamp: "Sunday", avatar: "user1.png" },
    Chat { id: 9, username: "Ana Leddie", preview: Preview::Text("Thanks!"), timestamp: "Sunday", avatar: "user2.png" },
    Chat { id: 10, username: "Adam Adler", preview: Preview::Text("Let's meet at 3pm"), timestamp: "12/04", avatar: "user3.png" },
    Chat { id: 11, username: "Gabriel Hayes", preview: Preview::Audio, timestamp: "12/03", avatar: "user4.png" },
    Chat { id: 12, username: "Eric Ford", preview: Preview::Text("I'm using WeChat"), timestamp: "12/01", avatar: "user5.png" },
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

/// Avatar for an incoming message in `chat_id` — the other party's.
pub fn peer_avatar(chat_id: u64) -> &'static str {
    chat(chat_id).map(|c| c.avatar).unwrap_or("default_avatar.png")
}

/// The user's own avatar, on outgoing messages.
pub const MY_AVATAR: &str = "default_avatar.png";

/// Tab bar icons, from the reference app's `resources/icons`.
pub const TAB_ICONS: &[(&str, &str)] = &[
    ("Chats", "chat.svg"),
    ("Contacts", "contacts.svg"),
    ("Discover", "discover.svg"),
    ("Me", "me.svg"),
];

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

/// The fixed entries above the contacts list, with the reference app's icons.
pub const CONTACT_ACTIONS: &[(&str, &str)] = &[
    ("New Friends", "new_friends.png"),
    ("Group Chats", "group_chats.png"),
    ("Tags", "tags.png"),
    ("Official Accounts", "official_accounts.png"),
    ("WeCom Contacts", "wecom_contacts.png"),
];

/// Discover screen, exactly the reference app's six entries and icons.
pub const DISCOVER: &[(&str, &str)] = &[
    ("Moments", "moments.png"),
    ("Scan", "scan.png"),
    ("Shake", "shake.png"),
    ("Search", "search.png"),
    ("People Nearby", "people_nearby.png"),
    ("Mini Programs", "mini_programs.png"),
];

/// Profile screen entries and icons, as the reference app assigns them.
pub const PROFILE: &[(&str, &str)] = &[
    ("Favorites", "favorites.png"),
    ("My Posts", "my-posts.png"),
    ("Stickers", "sticker-gallery.png"),
    ("Settings", "settings.png"),
];

/// Moments feed: (author, body, avatar, photo or "").
pub const MOMENTS: &[(&str, &str, &str, &str)] = &[
    ("Olive Yew", "体議速人幅触無持編聞組込", "user1.png", "post1.jpg"),
    ("John Doe", "減活乗治外進", "user2.png", ""),
    ("Peg Legge", "福読併棋一御質慰", "user3.png", "post2.jpg"),
    ("Barb Akew", "嶋可済政実玉全強無示餌", "user4.png", ""),
    ("Chris P. Bacon", "消再野誰強心無嶋可済実玉全示餌", "user5.png", "post1.jpg"),
    ("WeChat Team", "Welcome to WeChat Moments", "wechat_avatar.png", ""),
    ("Andrew Lin", "体議速人幅触無持編聞組込", "user6.png", "post2.jpg"),
    ("Ana Leddie", "減活乗治外進", "user2.png", ""),
];
