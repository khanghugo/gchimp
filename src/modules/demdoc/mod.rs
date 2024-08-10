pub mod change_map;
pub mod kz_stats;

#[macro_export]
macro_rules! wrap_message {
    ($svc:ident, $msg:ident) => {{
        use dem::types::EngineMessage;
        use dem::types::NetMessage;

        let huh = EngineMessage::$svc($msg);
        let hah = NetMessage::EngineMessage(Box::new(huh));
        hah
    }};
}

#[repr(u16)]
pub enum Buttons {
    Attack = 1 << 0,
    Jump = 1 << 1,
    Duck = 1 << 2,
    Forward = 1 << 3,
    Back = 1 << 4,
    Use = 1 << 5,
    Cancel = 1 << 6,
    Left = 1 << 7,
    Right = 1 << 8,
    MoveLeft = 1 << 9,
    MoveRight = 1 << 10,
    Attack2 = 1 << 11,
    Run = 1 << 12,
    Reload = 1 << 13,
    Alt1 = 1 << 14,
    Score = 1 << 15,
}
