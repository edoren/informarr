// diesel::table! {
//     users (id) {
//         id -> Int4,
//         #[max_length = 255]
//         username -> Varchar,
//         #[max_length = 255]
//         email -> Varchar,
//         #[max_length = 255]
//         discord_id -> Varchar,
//         #[max_length = 255]
//         telegram_chat_id -> Varchar,
//     }
// }

// diesel::table! {
//     requests (id) {
//         id -> Int4,
//         remote_id -> Varchar,
//         user_id -> Int4,
//         title -> Varchar,
//         body -> Text,
//     }
// }

// diesel::joinable!(requests-> users(user_id));
