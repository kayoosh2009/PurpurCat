pub const SPRITES_INTRO: &[&str] = &[
    "cat/sleep_anim/1.png",
    "cat/sleep_anim/2.png",
    "cat/sleep_anim/3.png",
    "cat/sleep_anim/4.png",
];

// Зацикленная часть сна: 4 → 5 → 4 → 5 → ...
pub const SPRITES_LOOP: &[&str] = &["cat/sleep_anim/4.png", "cat/sleep_anim/5.png"];

// Пробуждение (воспроизводится один раз): 4 → 3 → 2 → 1
pub const SPRITES_WAKE: &[&str] = &[
    "cat/sleep_anim/4.png",
    "cat/sleep_anim/3.png",
    "cat/sleep_anim/2.png",
    "cat/sleep_anim/1.png",
];
